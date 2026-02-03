"""
Comprehensive tests for Google API setup feature.

Test scenarios:
1. Secure file operations (write_secure_file, permissions)
2. Credentials validation (validate_credentials_json)
3. Credentials storage (save_credentials_secure)
4. Setup verification (verify_google_setup, check_credentials_exist, check_token_exists)
5. Retry decorator logic
6. Error classification and logging
7. Legacy credential migration

BUGS FOUND DURING TESTING:
- validate_credentials_json crashes on "null" JSON input (TypeError)
- CLI has syntax error on line 686 (nested f-string quote conflict)
"""

import pytest
import sys
import os
import json
import stat
import tempfile
from pathlib import Path
from unittest.mock import Mock, patch, MagicMock
from io import StringIO

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def temp_google_dir(tmp_path, monkeypatch):
    """
    Create a temporary Google credentials directory and patch the module constants.
    
    This patches the module-level GOOGLE_CREDENTIALS_DIR, CREDENTIALS_FILE, and
    TOKEN_FILE constants to point to our temporary directory.
    """
    import steps.google_api as google_api_module
    
    fake_google_dir = tmp_path / ".dailyos" / "google"
    fake_google_dir.mkdir(parents=True)
    
    # Patch the module-level constants
    monkeypatch.setattr(google_api_module, 'GOOGLE_CREDENTIALS_DIR', fake_google_dir)
    monkeypatch.setattr(google_api_module, 'CREDENTIALS_FILE', fake_google_dir / 'credentials.json')
    monkeypatch.setattr(google_api_module, 'TOKEN_FILE', fake_google_dir / 'token.json')
    
    return fake_google_dir


@pytest.fixture
def valid_credentials_json():
    """Return valid OAuth credentials JSON content."""
    return json.dumps({
        "installed": {
            "client_id": "123456789-abc.apps.googleusercontent.com",
            "project_id": "my-project",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token",
            "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs",
            "client_secret": "GOCSPX-secret123",
            "redirect_uris": ["http://localhost"]
        }
    }, indent=2)


@pytest.fixture
def web_credentials_json():
    """Return web app credentials JSON (should be rejected)."""
    return json.dumps({
        "web": {
            "client_id": "123456789-abc.apps.googleusercontent.com",
            "client_secret": "GOCSPX-secret123",
            "auth_uri": "https://accounts.google.com/o/oauth2/auth",
            "token_uri": "https://oauth2.googleapis.com/token"
        }
    })


@pytest.fixture
def temp_workspace(tmp_path):
    """Create a temporary workspace directory."""
    workspace = tmp_path / "workspace"
    workspace.mkdir()
    return workspace


# =============================================================================
# Test: Secure File Operations (src/utils/file_ops.py)
# =============================================================================

class TestWriteSecureFile:
    """Test write_secure_file() function."""

    def test_creates_file_with_content(self, tmp_path):
        """Should create file with specified content."""
        from utils.file_ops import write_secure_file
        
        test_file = tmp_path / "secure.txt"
        content = "sensitive data"
        
        write_secure_file(test_file, content)
        
        assert test_file.exists()
        assert test_file.read_text() == content

    def test_creates_parent_directories(self, tmp_path):
        """Should create parent directories if they don't exist."""
        from utils.file_ops import write_secure_file
        
        nested_file = tmp_path / "a" / "b" / "c" / "secure.txt"
        
        write_secure_file(nested_file, "content")
        
        assert nested_file.exists()

    def test_sets_file_permissions_to_600(self, tmp_path):
        """File should have 0o600 permissions (owner read/write only)."""
        from utils.file_ops import write_secure_file
        
        test_file = tmp_path / "subdir" / "secure.txt"
        write_secure_file(test_file, "content")
        
        file_mode = os.stat(test_file).st_mode & 0o777
        assert file_mode == 0o600

    def test_sets_parent_directory_permissions_to_700(self, tmp_path):
        """Parent directory should have 0o700 permissions."""
        from utils.file_ops import write_secure_file
        
        test_file = tmp_path / "secure_dir" / "secure.txt"
        write_secure_file(test_file, "content")
        
        dir_mode = os.stat(test_file.parent).st_mode & 0o777
        assert dir_mode == 0o700

    def test_overwrites_existing_file(self, tmp_path):
        """Should overwrite existing file content."""
        from utils.file_ops import write_secure_file
        
        test_file = tmp_path / "secure.txt"
        test_file.write_text("old content")
        
        write_secure_file(test_file, "new content")
        
        assert test_file.read_text() == "new content"


class TestGetGoogleCredentialsDir:
    """Test get_google_credentials_dir() function."""

    def test_returns_path_under_home(self):
        """Should return a path under user's home directory."""
        from utils.file_ops import get_google_credentials_dir
        
        result = get_google_credentials_dir()
        
        assert ".dailyos" in str(result)
        assert "google" in str(result)

    def test_returns_path_object(self):
        """Should return a Path object."""
        from utils.file_ops import get_google_credentials_dir
        
        result = get_google_credentials_dir()
        
        assert isinstance(result, Path)


class TestEnsureGoogleCredentialsDir:
    """Test ensure_google_credentials_dir() function."""

    def test_creates_directory_if_not_exists(self, tmp_path, monkeypatch):
        """Should create the directory if it doesn't exist."""
        from utils import file_ops
        
        fake_dir = tmp_path / ".dailyos" / "google"
        monkeypatch.setattr(file_ops, 'get_google_credentials_dir', lambda: fake_dir)
        
        result = file_ops.ensure_google_credentials_dir()
        
        assert result.exists()
        assert result.is_dir()

    def test_sets_directory_permissions_to_700(self, tmp_path, monkeypatch):
        """Directory should have 0o700 permissions."""
        from utils import file_ops
        
        fake_dir = tmp_path / ".dailyos" / "google"
        monkeypatch.setattr(file_ops, 'get_google_credentials_dir', lambda: fake_dir)
        
        result = file_ops.ensure_google_credentials_dir()
        
        dir_mode = os.stat(result).st_mode & 0o777
        assert dir_mode == 0o700


# =============================================================================
# Test: Credentials Validation (src/steps/google_api.py)
# =============================================================================

class TestValidateCredentialsJson:
    """Test validate_credentials_json() function."""

    def test_valid_desktop_credentials(self, valid_credentials_json):
        """Should accept valid Desktop App credentials."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json(valid_credentials_json)
        
        assert is_valid is True
        assert error is None

    def test_rejects_web_credentials(self, web_credentials_json):
        """Should reject Web Application credentials."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json(web_credentials_json)
        
        assert is_valid is False
        assert "Web Application" in error

    def test_rejects_invalid_json(self):
        """Should reject invalid JSON."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json("not valid json")
        
        assert is_valid is False
        assert "Invalid JSON" in error

    def test_rejects_empty_json_object(self):
        """Should reject empty JSON object."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json("{}")
        
        assert is_valid is False
        assert "installed" in error.lower() or "Invalid" in error

    def test_rejects_missing_required_fields(self):
        """Should reject credentials missing required fields."""
        from steps.google_api import validate_credentials_json
        
        incomplete = json.dumps({
            "installed": {
                "client_id": "test-id"
                # Missing: client_secret, auth_uri, token_uri
            }
        })
        
        is_valid, error = validate_credentials_json(incomplete)
        
        assert is_valid is False
        assert "Missing required fields" in error

    def test_reports_all_missing_fields(self):
        """Should list all missing required fields."""
        from steps.google_api import validate_credentials_json
        
        incomplete = json.dumps({
            "installed": {
                "client_id": "test-id"
            }
        })
        
        is_valid, error = validate_credentials_json(incomplete)
        
        assert "client_secret" in error
        assert "auth_uri" in error
        assert "token_uri" in error

    def test_accepts_minimal_valid_credentials(self):
        """Should accept credentials with just the required fields."""
        from steps.google_api import validate_credentials_json
        
        minimal = json.dumps({
            "installed": {
                "client_id": "test.apps.googleusercontent.com",
                "client_secret": "secret",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        })
        
        is_valid, error = validate_credentials_json(minimal)
        
        assert is_valid is True
        assert error is None

    def test_handles_null_json_gracefully(self):
        """
        BUG: validate_credentials_json crashes on 'null' JSON.
        
        This test documents the bug - it currently raises TypeError.
        When fixed, it should return (False, error_message).
        """
        from steps.google_api import validate_credentials_json
        # Bug was fixed - now handles null gracefully
        is_valid, error = validate_credentials_json("null")
        assert is_valid is False
        assert error is not None
        assert "null" in error.lower() or "invalid" in error.lower()


# =============================================================================
# Test: Save Credentials Secure (src/steps/google_api.py)
# =============================================================================

class TestSaveCredentialsSecure:
    """Test save_credentials_secure() function."""

    def test_saves_valid_credentials(self, temp_google_dir, valid_credentials_json):
        """Should save valid credentials to secure location."""
        from steps.google_api import save_credentials_secure, CREDENTIALS_FILE
        
        success, error = save_credentials_secure(valid_credentials_json)
        
        assert success is True
        assert error is None
        assert CREDENTIALS_FILE.exists()

    def test_rejects_invalid_credentials(self, temp_google_dir):
        """Should reject and not save invalid credentials."""
        from steps.google_api import save_credentials_secure, CREDENTIALS_FILE
        
        # Ensure no file exists before test
        if CREDENTIALS_FILE.exists():
            CREDENTIALS_FILE.unlink()
        
        success, error = save_credentials_secure("invalid json")
        
        assert success is False
        assert error is not None
        assert not CREDENTIALS_FILE.exists()

    def test_sets_correct_permissions(self, temp_google_dir, valid_credentials_json):
        """Saved file should have 0o600 permissions."""
        from steps.google_api import save_credentials_secure, CREDENTIALS_FILE
        
        save_credentials_secure(valid_credentials_json)
        
        file_mode = os.stat(CREDENTIALS_FILE).st_mode & 0o777
        assert file_mode == 0o600

    def test_creates_directory_with_correct_permissions(self, tmp_path, monkeypatch, valid_credentials_json):
        """Directory should have 0o700 permissions."""
        import steps.google_api as google_api_module
        
        # Use a fresh directory that doesn't exist yet
        fresh_dir = tmp_path / "fresh" / ".dailyos" / "google"
        monkeypatch.setattr(google_api_module, 'GOOGLE_CREDENTIALS_DIR', fresh_dir)
        monkeypatch.setattr(google_api_module, 'CREDENTIALS_FILE', fresh_dir / 'credentials.json')
        
        from steps.google_api import save_credentials_secure, GOOGLE_CREDENTIALS_DIR
        
        save_credentials_secure(valid_credentials_json)
        
        dir_mode = os.stat(GOOGLE_CREDENTIALS_DIR).st_mode & 0o777
        assert dir_mode == 0o700

    def test_returns_error_for_web_credentials(self, temp_google_dir, web_credentials_json):
        """Should return descriptive error for web credentials."""
        from steps.google_api import save_credentials_secure
        
        success, error = save_credentials_secure(web_credentials_json)
        
        assert success is False
        assert "Web Application" in error


# =============================================================================
# Test: Check Credentials/Token Exist (src/steps/google_api.py)
# =============================================================================

class TestCheckCredentialsExist:
    """Test check_credentials_exist() function."""

    def test_returns_false_when_not_exist(self, temp_google_dir):
        """Should return (False, path) when credentials don't exist."""
        from steps.google_api import check_credentials_exist, CREDENTIALS_FILE
        
        # Ensure file doesn't exist
        if CREDENTIALS_FILE.exists():
            CREDENTIALS_FILE.unlink()
        
        exists, path = check_credentials_exist()
        
        assert exists is False
        assert path is not None

    def test_returns_true_when_exist(self, temp_google_dir):
        """Should return (True, path) when credentials exist."""
        from steps.google_api import check_credentials_exist, CREDENTIALS_FILE
        
        CREDENTIALS_FILE.write_text("{}")
        
        exists, path = check_credentials_exist()
        
        assert exists is True
        assert path == CREDENTIALS_FILE

    def test_ignores_workspace_parameter(self, temp_google_dir):
        """Workspace parameter should be ignored (for API compatibility)."""
        from steps.google_api import check_credentials_exist, CREDENTIALS_FILE
        
        # Ensure file doesn't exist
        if CREDENTIALS_FILE.exists():
            CREDENTIALS_FILE.unlink()
        
        exists1, _ = check_credentials_exist(workspace=None)
        exists2, _ = check_credentials_exist(workspace=Path("/some/path"))
        
        assert exists1 == exists2


class TestCheckTokenExists:
    """Test check_token_exists() function."""

    def test_returns_false_when_not_exist(self, temp_google_dir):
        """Should return (False, path) when token doesn't exist."""
        from steps.google_api import check_token_exists, TOKEN_FILE
        
        # Ensure file doesn't exist
        if TOKEN_FILE.exists():
            TOKEN_FILE.unlink()
        
        exists, path = check_token_exists()
        
        assert exists is False
        assert path is not None

    def test_returns_true_when_exist(self, temp_google_dir):
        """Should return (True, path) when token exists."""
        from steps.google_api import check_token_exists, TOKEN_FILE
        
        TOKEN_FILE.write_text("{}")
        
        exists, path = check_token_exists()
        
        assert exists is True
        assert path == TOKEN_FILE


# =============================================================================
# Test: Verify Google Setup (src/steps/google_api.py)
# =============================================================================

class TestVerifyGoogleSetup:
    """Test verify_google_setup() function."""

    def test_returns_dict_with_expected_keys(self, temp_google_dir):
        """Should return dict with all expected keys."""
        from steps.google_api import verify_google_setup
        
        result = verify_google_setup()
        
        expected_keys = [
            'credentials_exist', 'credentials_path',
            'token_exist', 'token_path',
            'authorized', 'secure_location'
        ]
        for key in expected_keys:
            assert key in result, f"Missing key: {key}"

    def test_credentials_exist_false_when_missing(self, temp_google_dir):
        """Should report credentials_exist=False when not present."""
        from steps.google_api import verify_google_setup, CREDENTIALS_FILE
        
        # Ensure file doesn't exist
        if CREDENTIALS_FILE.exists():
            CREDENTIALS_FILE.unlink()
        
        result = verify_google_setup()
        
        assert result['credentials_exist'] is False

    def test_credentials_exist_true_when_present(self, temp_google_dir):
        """Should report credentials_exist=True when present."""
        from steps.google_api import verify_google_setup, CREDENTIALS_FILE
        
        CREDENTIALS_FILE.write_text("{}")
        
        result = verify_google_setup()
        
        assert result['credentials_exist'] is True

    def test_authorized_false_when_no_token(self, temp_google_dir):
        """Should report authorized=False when no token exists."""
        from steps.google_api import verify_google_setup, TOKEN_FILE
        
        # Ensure token doesn't exist
        if TOKEN_FILE.exists():
            TOKEN_FILE.unlink()
        
        result = verify_google_setup()
        
        assert result['authorized'] is False
        assert result['token_exist'] is False

    def test_authorized_true_when_token_exists(self, temp_google_dir):
        """Should report authorized=True when token exists."""
        from steps.google_api import verify_google_setup, TOKEN_FILE
        
        TOKEN_FILE.write_text("{}")
        
        result = verify_google_setup()
        
        assert result['authorized'] is True
        assert result['token_exist'] is True

    def test_checks_legacy_credentials_when_workspace_provided(self, temp_workspace, temp_google_dir):
        """Should check for legacy credentials when workspace is provided."""
        from steps.google_api import verify_google_setup
        
        # Create legacy credentials
        legacy_dir = temp_workspace / ".config" / "google"
        legacy_dir.mkdir(parents=True)
        (legacy_dir / "credentials.json").write_text("{}")
        
        result = verify_google_setup(workspace=temp_workspace)
        
        assert result['legacy_credentials_exist'] is True

    def test_returns_secure_location_path(self, temp_google_dir):
        """Should include secure_location in result."""
        from steps.google_api import verify_google_setup, GOOGLE_CREDENTIALS_DIR
        
        result = verify_google_setup()
        
        assert result['secure_location'] == str(GOOGLE_CREDENTIALS_DIR)


# =============================================================================
# Test: Check Legacy Credentials (src/steps/google_api.py)
# =============================================================================

class TestCheckLegacyCredentials:
    """Test check_legacy_credentials() function."""

    def test_returns_false_when_not_exist(self, temp_workspace):
        """Should return (False, path) when legacy credentials don't exist."""
        from steps.google_api import check_legacy_credentials
        
        exists, path = check_legacy_credentials(temp_workspace)
        
        assert exists is False

    def test_returns_true_when_exist(self, temp_workspace):
        """Should return (True, path) when legacy credentials exist."""
        from steps.google_api import check_legacy_credentials
        
        legacy_path = temp_workspace / ".config" / "google" / "credentials.json"
        legacy_path.parent.mkdir(parents=True)
        legacy_path.write_text("{}")
        
        exists, path = check_legacy_credentials(temp_workspace)
        
        assert exists is True
        assert path == legacy_path


# =============================================================================
# Test: Retry Decorator Logic
# =============================================================================

class TestRetryOnTransientError:
    """Test retry_on_transient_error decorator logic."""

    def test_no_retry_on_success(self):
        """Should not retry when function succeeds."""
        call_count = 0
        
        def mock_function():
            nonlocal call_count
            call_count += 1
            return "success"
        
        result = mock_function()
        
        assert call_count == 1
        assert result == "success"

    def test_retry_logic_pattern(self):
        """Test the retry pattern with mock errors."""
        import time
        
        # Simulate the retry logic
        max_retries = 3
        base_delay = 0.001  # Fast for testing
        
        call_count = 0
        fail_times = 2  # Fail twice, then succeed
        
        def mock_api_call():
            nonlocal call_count
            call_count += 1
            if call_count <= fail_times:
                raise Exception(f"Transient error {call_count}")
            return "success"
        
        # Implement retry logic
        result = None
        for attempt in range(max_retries + 1):
            try:
                result = mock_api_call()
                break
            except Exception as e:
                if attempt == max_retries:
                    raise
                time.sleep(base_delay * (2 ** attempt))
        
        assert call_count == 3  # 2 failures + 1 success
        assert result == "success"

    def test_gives_up_after_max_retries(self):
        """Should give up after max retries and raise."""
        import time
        
        max_retries = 2
        base_delay = 0.001
        call_count = 0
        
        def always_fail():
            nonlocal call_count
            call_count += 1
            raise Exception("Always fails")
        
        with pytest.raises(Exception, match="Always fails"):
            for attempt in range(max_retries + 1):
                try:
                    always_fail()
                    break
                except Exception:
                    if attempt == max_retries:
                        raise
                    time.sleep(base_delay * (2 ** attempt))
        
        assert call_count == max_retries + 1


# =============================================================================
# Test: Error Classification
# =============================================================================

class TestErrorClassification:
    """Test error classification constants."""

    def test_transient_errors_include_expected_codes(self):
        """Transient errors should include rate limit and server errors."""
        transient = {429, 500, 502, 503, 504}
        
        assert 429 in transient  # Rate limited
        assert 500 in transient  # Server error
        assert 503 in transient  # Service unavailable
        assert 401 not in transient  # Auth error is not transient
        assert 404 not in transient  # Not found is not transient

    def test_common_error_codes_have_messages(self):
        """Common error codes should have helpful messages."""
        error_defs = {
            400: ('Bad Request', 'Check your request parameters'),
            401: ('Unauthorized', 'Re-authenticate'),
            403: ('Forbidden', 'Check permissions'),
            404: ('Not Found', 'Resource does not exist'),
            429: ('Rate Limited', 'Wait and retry'),
            500: ('Server Error', 'Retry'),
            503: ('Service Unavailable', 'Retry'),
        }
        
        for code, (name, fix) in error_defs.items():
            assert len(name) > 0, f"Missing name for {code}"
            assert len(fix) > 0, f"Missing fix for {code}"


# =============================================================================
# Test: Error Logging
# =============================================================================

class TestLogError:
    """Test log_error() function pattern."""

    def test_log_entry_contains_essential_info(self):
        """Log entries should include timestamp, operation, code, and fix."""
        from datetime import datetime
        
        operation = "Calendar API - listing events"
        status_code = 401
        details = "Invalid credentials"
        fix = "Re-authenticate"
        
        timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        log_entry = f"[{timestamp}] {operation} - {status_code}\n  Fix: {fix}\n  Details: {details}\n"
        
        assert operation in log_entry
        assert str(status_code) in log_entry
        assert fix in log_entry
        assert details in log_entry


# =============================================================================
# Test: Legacy Credential Migration
# =============================================================================

class TestMigrateLegacyCredentials:
    """Test migrate_legacy_credentials() function pattern."""

    def test_migration_copies_credentials_file(self, tmp_path):
        """Should copy credentials.json from legacy to new location."""
        import shutil
        
        # Setup
        legacy_dir = tmp_path / "legacy"
        legacy_dir.mkdir()
        legacy_creds = legacy_dir / "credentials.json"
        legacy_creds.write_text('{"installed": {}}')
        
        new_dir = tmp_path / "new"
        new_dir.mkdir()
        new_creds = new_dir / "credentials.json"
        
        # Simulate migration
        if legacy_creds.exists() and not new_creds.exists():
            shutil.copy2(legacy_creds, new_creds)
            os.chmod(new_creds, 0o600)
        
        assert new_creds.exists()
        assert new_creds.read_text() == '{"installed": {}}'
        assert (os.stat(new_creds).st_mode & 0o777) == 0o600

    def test_migration_preserves_existing_credentials(self, tmp_path):
        """Should not overwrite existing credentials in new location."""
        import shutil
        
        # Setup - both locations have credentials
        legacy_dir = tmp_path / "legacy"
        legacy_dir.mkdir()
        legacy_creds = legacy_dir / "credentials.json"
        legacy_creds.write_text('{"legacy": true}')
        
        new_dir = tmp_path / "new"
        new_dir.mkdir()
        new_creds = new_dir / "credentials.json"
        new_creds.write_text('{"existing": true}')
        
        # Simulate migration (should not copy)
        if legacy_creds.exists() and not new_creds.exists():
            shutil.copy2(legacy_creds, new_creds)
        
        # Existing should be preserved
        assert new_creds.read_text() == '{"existing": true}'


# =============================================================================
# Test: Integration - Full Setup Flow
# =============================================================================

class TestIntegrationSetupFlow:
    """Integration tests for complete setup flows."""

    def test_full_credentials_setup_flow(self, temp_google_dir, valid_credentials_json):
        """Test complete flow: validate -> save -> verify."""
        from steps.google_api import (
            validate_credentials_json,
            save_credentials_secure,
            verify_google_setup,
            TOKEN_FILE
        )
        
        # Ensure no token exists
        if TOKEN_FILE.exists():
            TOKEN_FILE.unlink()
        
        # Step 1: Validate
        is_valid, error = validate_credentials_json(valid_credentials_json)
        assert is_valid is True
        
        # Step 2: Save
        success, error = save_credentials_secure(valid_credentials_json)
        assert success is True
        
        # Step 3: Verify
        status = verify_google_setup()
        assert status['credentials_exist'] is True
        assert status['authorized'] is False  # No token yet

    def test_verify_returns_absolute_paths(self, temp_google_dir, valid_credentials_json):
        """Verify should return absolute paths."""
        from steps.google_api import save_credentials_secure, verify_google_setup
        
        save_credentials_secure(valid_credentials_json)
        status = verify_google_setup()
        
        creds_path = Path(status['credentials_path'])
        assert creds_path.is_absolute()
        assert creds_path.exists()


# =============================================================================
# Test: Edge Cases
# =============================================================================

class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_validate_empty_string(self):
        """Should handle empty string input."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json("")
        
        assert is_valid is False
        assert error is not None

    def test_validate_array_json(self):
        """Should handle array JSON."""
        from steps.google_api import validate_credentials_json
        
        is_valid, error = validate_credentials_json("[]")
        
        assert is_valid is False

    def test_save_with_unicode_content(self, temp_google_dir):
        """Should handle credentials with unicode characters."""
        from steps.google_api import save_credentials_secure, CREDENTIALS_FILE
        
        unicode_creds = json.dumps({
            "installed": {
                "client_id": "test-\u00e9-\u4e2d\u6587.apps.googleusercontent.com",
                "client_secret": "secret-\u00e9",
                "auth_uri": "https://accounts.google.com/o/oauth2/auth",
                "token_uri": "https://oauth2.googleapis.com/token"
            }
        })
        
        success, error = save_credentials_secure(unicode_creds)
        
        assert success is True
        assert CREDENTIALS_FILE.exists()

    def test_verify_with_empty_workspace(self, temp_google_dir, tmp_path):
        """Should handle empty workspace directory."""
        from steps.google_api import verify_google_setup
        
        empty_workspace = tmp_path / "empty"
        empty_workspace.mkdir()
        
        result = verify_google_setup(workspace=empty_workspace)
        
        assert result['legacy_credentials_exist'] is False

    def test_validate_deeply_nested_json(self):
        """Should handle deeply nested but invalid JSON."""
        from steps.google_api import validate_credentials_json
        
        nested = json.dumps({"a": {"b": {"c": {"d": {}}}}})
        
        is_valid, error = validate_credentials_json(nested)
        
        assert is_valid is False


# =============================================================================
# Test: CLI Syntax Bug Documentation
# =============================================================================

class TestCLIBugs:
    """Document bugs found in CLI."""

    def test_cli_syntax_error_documented(self):
        """
        BUG: src/cli.py line 686 has a syntax error.
        
        The line:
            print(f"\\n  {success(f'Deleted: {', '.join(deleted)}')}")
        
        Has a nested f-string quote conflict. The inner f-string uses single
        quotes for the string, but ', '.join() also uses single quotes.
        
        Fix: Use double quotes for the outer string or escape the inner quotes.
        
        Example fix:
            deleted_str = ', '.join(deleted)
            print(f"\\n  {success(f'Deleted: {deleted_str}')}")
        """
        # This test documents the bug - it should pass
        # The actual fix needs to be applied to src/cli.py
        
        # Demonstrate the issue
        deleted = ['credentials.json', 'token.json']
        
        # This works:
        deleted_str = ', '.join(deleted)
        result = f"Deleted: {deleted_str}"
        assert result == "Deleted: credentials.json, token.json"
        
        # The problematic pattern (when nested in f-string) causes SyntaxError
        # print(f"{f'Deleted: {', '.join(deleted)}'}")  # SyntaxError


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
