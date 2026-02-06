use std::fs;
use std::path::Path;

use crate::types::{
    Action, ActionStatus, ActionWithContext, AlertSeverity, DayOverview, DayStats, Email,
    EmailDetail, EmailPriority, EmailStats, EmailSummaryData, EnergyNotes, FocusData,
    FocusPriority, FullMeetingPrep, HygieneAlert, InboxFile, InboxFileType, Meeting, MeetingPrep,
    MeetingType, PrepStatus, Priority, SourceReference, Stakeholder, TimeBlock, WeekActionSummary,
    WeekDay, WeekMeeting, WeekOverview,
};

/// Parse the overview.md file into a DayOverview struct
/// Handles both legacy format and DailyOS format (00-overview.md)
pub fn parse_overview(path: &Path) -> Result<DayOverview, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read overview: {}", e))?;

    let mut greeting = String::from("Good morning");
    let mut date = String::new();
    let mut summary = String::new();
    let mut focus: Option<String> = None;

    let mut in_frontmatter = false;
    let mut in_focus = false;
    let mut in_summary = false;
    let mut in_schedule = false;
    let mut meeting_count = 0;
    let mut customer_meeting_count = 0;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Handle YAML frontmatter
        if line_trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }

        if in_frontmatter {
            if let Some(value) = line_trimmed.strip_prefix("date:") {
                date = format_date(value.trim().trim_matches('"'));
            } else if let Some(value) = line_trimmed.strip_prefix("greeting:") {
                greeting = value.trim().trim_matches('"').to_string();
            }
            continue;
        }

        // DailyOS format: "# Today: Wednesday, February 04, 2026"
        if line_trimmed.starts_with("# Today:") {
            if let Some(date_part) = line_trimmed.strip_prefix("# Today:") {
                // Extract just the date part (e.g., "February 04")
                let parts: Vec<&str> = date_part.trim().split(", ").collect();
                if parts.len() >= 2 {
                    // Format: "Wednesday, February 04, 2026" -> "February 04"
                    let date_str = parts[1..].join(", ");
                    let date_components: Vec<&str> = date_str.split(' ').collect();
                    if date_components.len() >= 2 {
                        date = format!("{} {}", date_components[0], date_components[1].trim_end_matches(','));
                    }
                }
                // Determine greeting based on current time
                greeting = get_time_based_greeting();
            }
            continue;
        }

        // Handle sections
        if line_trimmed.starts_with("## Schedule") {
            in_schedule = true;
            in_focus = false;
            in_summary = false;
            continue;
        }

        if line_trimmed.starts_with("## Focus") || line_trimmed.starts_with("## focus") {
            in_focus = true;
            in_schedule = false;
            in_summary = false;
            continue;
        }

        if line_trimmed.starts_with("# ") || line_trimmed.starts_with("## ") {
            in_focus = false;
            in_schedule = false;
            in_summary = line_trimmed.contains("Overview") || line_trimmed.contains("Summary");
            continue;
        }

        // Count meetings from schedule table
        if in_schedule && line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
            // Skip header row
            if line_trimmed.contains("Time") && line_trimmed.contains("Event") {
                continue;
            }
            meeting_count += 1;
            if line_trimmed.contains("Customer") {
                customer_meeting_count += 1;
            }
        }

        // Capture content
        if in_focus && !line_trimmed.is_empty() && !line_trimmed.starts_with('|') {
            if line_trimmed.starts_with("- [ ]") {
                // Extract focus item
                let item = line_trimmed.strip_prefix("- [ ]").unwrap_or("").trim();
                focus = Some(item.to_string());
            } else if focus.is_none() {
                focus = Some(line_trimmed.to_string());
            }
        } else if in_summary && !line_trimmed.is_empty() {
            if summary.is_empty() {
                summary = line_trimmed.to_string();
            } else {
                summary.push(' ');
                summary.push_str(line_trimmed);
            }
        }
    }

    // Generate summary from meeting count if not found
    if summary.is_empty() {
        if meeting_count > 0 {
            summary = format!(
                "You have {} meeting{} today{}.",
                meeting_count,
                if meeting_count == 1 { "" } else { "s" },
                if customer_meeting_count > 0 {
                    format!(", including {} customer call{}",
                        customer_meeting_count,
                        if customer_meeting_count == 1 { "" } else { "s" })
                } else {
                    String::new()
                }
            );
        } else {
            summary = "Your day is ready.".to_string();
        }
    }

    // Use a default date if not found
    if date.is_empty() {
        date = chrono_date_fallback();
    }

    Ok(DayOverview {
        greeting,
        date,
        summary,
        focus,
    })
}

/// Get greeting based on current time
fn get_time_based_greeting() -> String {
    use std::time::SystemTime;

    // Simple hour extraction (rough approximation)
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| ((d.as_secs() % 86400) / 3600) as u32)
        .unwrap_or(12);

    // Adjust for EST (UTC-5) roughly
    let hour = (now + 19) % 24; // UTC to EST adjustment

    if hour < 12 {
        "Good morning".to_string()
    } else if hour < 17 {
        "Good afternoon".to_string()
    } else {
        "Good evening".to_string()
    }
}

/// Format a date string (YYYY-MM-DD) to display format
fn format_date(date_str: &str) -> String {
    // Simple parsing for YYYY-MM-DD format
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let month = match parts[1] {
            "01" => "January",
            "02" => "February",
            "03" => "March",
            "04" => "April",
            "05" => "May",
            "06" => "June",
            "07" => "July",
            "08" => "August",
            "09" => "September",
            "10" => "October",
            "11" => "November",
            "12" => "December",
            _ => parts[1],
        };
        let day: u32 = parts[2].parse().unwrap_or(1);
        format!("{} {}", month, day)
    } else {
        date_str.to_string()
    }
}

/// Fallback date when not specified in file
fn chrono_date_fallback() -> String {
    // Use simple approach without chrono dependency
    "Today".to_string()
}

/// Parse the meetings.md file into a list of Meeting structs
pub fn parse_meetings(path: &Path) -> Result<Vec<Meeting>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read meetings: {}", e))?;

    let mut meetings = Vec::new();
    let mut current_meeting: Option<MeetingBuilder> = None;
    let mut in_prep = false;
    let mut current_prep_section: Option<String> = None;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Meeting header: ## 9:00 AM - Title
        if line_trimmed.starts_with("## ") {
            // Save previous meeting
            if let Some(builder) = current_meeting.take() {
                if let Some(meeting) = builder.build() {
                    meetings.push(meeting);
                }
            }

            if let Some((time, title)) = parse_meeting_header(line_trimmed) {
                current_meeting = Some(MeetingBuilder::new(time, title));
            }
            in_prep = false;
            current_prep_section = None;
            continue;
        }

        // Meeting metadata: type: customer, account: Acme Corp
        if let Some(ref mut builder) = current_meeting {
            if let Some(value) = line_trimmed.strip_prefix("type:") {
                builder.meeting_type = Some(parse_meeting_type(value.trim()));
            } else if let Some(value) = line_trimmed.strip_prefix("account:") {
                builder.account = Some(value.trim().to_string());
            } else if let Some(value) = line_trimmed.strip_prefix("end:") {
                builder.end_time = Some(value.trim().to_string());
            }

            // Prep section
            if line_trimmed.starts_with("### Prep") {
                in_prep = true;
                continue;
            }

            if in_prep {
                // Prep subsections: **Metrics**, **Risks**, etc.
                if line_trimmed.starts_with("**") {
                    let section = line_trimmed
                        .trim_start_matches("**")
                        .split("**")
                        .next()
                        .unwrap_or("")
                        .to_lowercase();

                    if section.starts_with("context") {
                        // Context is inline: **Context**: Some text
                        if let Some(value) = line_trimmed.split("**:").nth(1) {
                            builder.prep.context = Some(value.trim().to_string());
                        } else if let Some(value) = line_trimmed.split(": ").nth(1) {
                            builder.prep.context = Some(value.trim().to_string());
                        }
                        current_prep_section = None;
                    } else {
                        current_prep_section = Some(section);
                    }
                    continue;
                }

                // List items in prep sections
                if line_trimmed.starts_with("- ") || line_trimmed.starts_with("* ") {
                    let item = line_trimmed[2..].to_string();
                    if let Some(ref section) = current_prep_section {
                        match section.as_str() {
                            "metrics" => builder.prep.metrics.get_or_insert_with(Vec::new).push(item),
                            "risks" => builder.prep.risks.get_or_insert_with(Vec::new).push(item),
                            "wins" => builder.prep.wins.get_or_insert_with(Vec::new).push(item),
                            "actions" => builder.prep.actions.get_or_insert_with(Vec::new).push(item),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Don't forget the last meeting
    if let Some(builder) = current_meeting {
        if let Some(meeting) = builder.build() {
            meetings.push(meeting);
        }
    }

    Ok(meetings)
}

/// Parse meeting header line: "## 9:00 AM - Title"
fn parse_meeting_header(line: &str) -> Option<(String, String)> {
    let content = line.strip_prefix("## ")?.trim();

    // Try to split on " - " to separate time from title
    if let Some((time_part, title_part)) = content.split_once(" - ") {
        Some((time_part.trim().to_string(), title_part.trim().to_string()))
    } else {
        // Fallback: first word(s) that look like time, rest is title
        None
    }
}

fn parse_meeting_type(s: &str) -> MeetingType {
    match s.to_lowercase().as_str() {
        "customer" => MeetingType::Customer,
        "qbr" => MeetingType::Qbr,
        "training" => MeetingType::Training,
        "internal" => MeetingType::Internal,
        "team_sync" | "team-sync" => MeetingType::TeamSync,
        "one_on_one" | "one-on-one" | "1:1" => MeetingType::OneOnOne,
        "partnership" => MeetingType::Partnership,
        "all_hands" | "all-hands" => MeetingType::AllHands,
        "external" => MeetingType::External,
        "personal" => MeetingType::Personal,
        _ => MeetingType::Internal,
    }
}

struct MeetingBuilder {
    id: String,
    time: String,
    end_time: Option<String>,
    title: String,
    meeting_type: Option<MeetingType>,
    account: Option<String>,
    prep: MeetingPrep,
    prep_file: Option<String>,
    has_prep: bool,
}

impl MeetingBuilder {
    fn new(time: String, title: String) -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Self {
            id: id.to_string(),
            time,
            end_time: None,
            title,
            meeting_type: None,
            account: None,
            prep: MeetingPrep::default(),
            prep_file: None,
            has_prep: false,
        }
    }

    fn build(self) -> Option<Meeting> {
        let has_inline_prep = !self.prep.is_empty();
        Some(Meeting {
            id: self.id,
            time: self.time,
            end_time: self.end_time,
            title: self.title,
            meeting_type: self.meeting_type.unwrap_or(MeetingType::Internal),
            account: self.account,
            prep: if has_inline_prep {
                Some(self.prep)
            } else {
                None
            },
            is_current: None,
            prep_file: self.prep_file,
            has_prep: self.has_prep || has_inline_prep,
        })
    }
}

impl MeetingPrep {
    fn is_empty(&self) -> bool {
        self.metrics.is_none()
            && self.risks.is_none()
            && self.wins.is_none()
            && self.actions.is_none()
            && self.context.is_none()
    }
}

/// Parse the actions.md file into a list of Action structs
/// Handles both legacy format and DailyOS format (80-actions-due.md)
pub fn parse_actions(path: &Path) -> Result<Vec<Action>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read actions: {}", e))?;

    let mut actions = Vec::new();
    let mut id_counter = 1;
    let mut current_action: Option<ActionBuilder> = None;
    let mut in_context = false;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // DailyOS format: - [ ] **Title** - Account - Due: 2026-01-24 (X days overdue)
        // With nested context lines:
        //   - **Context**: ...
        //   - **Source**: ...

        // Check if this is a top-level action line
        if line_trimmed.starts_with("- [ ]") || line_trimmed.starts_with("- [x]") || line_trimmed.starts_with("- [X]") {
            // Save previous action
            if let Some(builder) = current_action.take() {
                actions.push(builder.build(id_counter));
                id_counter += 1;
            }

            let is_completed = line_trimmed.starts_with("- [x]") || line_trimmed.starts_with("- [X]");
            let rest = line_trimmed
                .strip_prefix("- [x] ")
                .or_else(|| line_trimmed.strip_prefix("- [X] "))
                .or_else(|| line_trimmed.strip_prefix("- [ ] "))
                .unwrap_or("");

            current_action = Some(parse_action_line(rest, is_completed));
            in_context = true;
            continue;
        }

        // Check for nested context/source lines (indented with 2 spaces or starting with "  -")
        if in_context && (line.starts_with("  ") || line.starts_with("\t")) {
            if let Some(ref mut builder) = current_action {
                let nested = line_trimmed.strip_prefix("- ").unwrap_or(line_trimmed);

                if let Some(ctx) = nested.strip_prefix("**Context**:") {
                    builder.context = Some(ctx.trim().to_string());
                } else if let Some(ctx) = nested.strip_prefix("**Context:**") {
                    builder.context = Some(ctx.trim().to_string());
                } else if let Some(src) = nested.strip_prefix("**Source**:") {
                    builder.source = Some(src.trim().to_string());
                } else if let Some(src) = nested.strip_prefix("**Source:**") {
                    builder.source = Some(src.trim().to_string());
                } else if !nested.is_empty() && builder.context.is_none() {
                    // Treat as context if no explicit prefix
                    builder.context = Some(nested.to_string());
                }
            }
            continue;
        }

        // Section headers reset context
        if line_trimmed.starts_with('#') || line_trimmed.is_empty() {
            in_context = false;
        }
    }

    // Don't forget the last action
    if let Some(builder) = current_action {
        actions.push(builder.build(id_counter));
    }

    Ok(actions)
}

/// Helper struct for building actions
struct ActionBuilder {
    title: String,
    account: Option<String>,
    due_date: Option<String>,
    priority: Priority,
    is_completed: bool,
    is_overdue: bool,
    days_overdue: Option<i32>,
    context: Option<String>,
    source: Option<String>,
}

impl ActionBuilder {
    fn build(self, id: usize) -> Action {
        Action {
            id: format!("a{}", id),
            title: self.title,
            account: self.account,
            due_date: self.due_date,
            priority: self.priority,
            status: if self.is_completed {
                ActionStatus::Completed
            } else {
                ActionStatus::Pending
            },
            is_overdue: if self.is_overdue { Some(true) } else { None },
            days_overdue: self.days_overdue,
            context: self.context,
            source: self.source,
        }
    }
}

/// Parse a single action line into an ActionBuilder
fn parse_action_line(line: &str, is_completed: bool) -> ActionBuilder {
    let mut title = line.to_string();
    let mut account: Option<String> = None;
    let mut due_date: Option<String> = None;
    let mut priority = Priority::P2;
    let mut is_overdue = false;
    let mut days_overdue: Option<i32> = None;

    // Check for bold title: **Title** - Account - Due: ...
    if let Some(start) = title.find("**") {
        if let Some(end) = title[start + 2..].find("**") {
            let bold_title = title[start + 2..start + 2 + end].to_string();
            title = bold_title;

            // Parse the rest after the bold title
            let rest = &line[start + 4 + end..];
            parse_action_metadata(rest, &mut account, &mut due_date, &mut is_overdue, &mut days_overdue);
        }
    } else {
        // Legacy format: P1: Title @Account due:2024-02-05
        // Check for priority prefix
        if let Some(r) = title.strip_prefix("P1:") {
            priority = Priority::P1;
            title = r.trim().to_string();
        } else if let Some(r) = title.strip_prefix("P2:") {
            priority = Priority::P2;
            title = r.trim().to_string();
        } else if let Some(r) = title.strip_prefix("P3:") {
            priority = Priority::P3;
            title = r.trim().to_string();
        }

        // Extract account (@Account)
        if let Some(at_pos) = title.find('@') {
            let after_at = &title[at_pos + 1..];
            let account_end = after_at
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_at.len());
            account = Some(after_at[..account_end].to_string());
            title = format!("{}{}", &title[..at_pos], &title[at_pos + 1 + account_end..])
                .trim()
                .to_string();
        }

        // Extract due date (due:YYYY-MM-DD)
        if let Some(due_pos) = title.find("due:") {
            let after_due = &title[due_pos + 4..];
            let due_end = after_due
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_due.len());
            let raw_due = &after_due[..due_end];
            due_date = Some(format_due_date(raw_due));
            title = format!("{}{}", &title[..due_pos], &title[due_pos + 4 + due_end..])
                .trim()
                .to_string();
        }

        // Check if overdue
        is_overdue = due_date
            .as_ref()
            .map(|d| d.contains("Yesterday") || d.contains("Overdue"))
            .unwrap_or(false);
    }

    ActionBuilder {
        title,
        account,
        due_date,
        priority,
        is_completed,
        is_overdue,
        days_overdue,
        context: None,
        source: None,
    }
}

/// Parse metadata from action line (account, due date, overdue status)
fn parse_action_metadata(
    rest: &str,
    account: &mut Option<String>,
    due_date: &mut Option<String>,
    is_overdue: &mut bool,
    days_overdue: &mut Option<i32>,
) {
    // Format: " - Account Name - Due: 2026-01-24 (11 days overdue)"
    let parts: Vec<&str> = rest.split(" - ").collect();

    for part in parts {
        let part = part.trim();

        // Check for Due: prefix
        if part.starts_with("Due:") {
            let date_part = part.strip_prefix("Due:").unwrap_or("").trim();

            // Check for overdue indicator
            if let Some(paren_pos) = date_part.find('(') {
                let raw_date = date_part[..paren_pos].trim();
                *due_date = Some(format_due_date(raw_date));

                // Extract days overdue
                let overdue_part = &date_part[paren_pos..];
                if overdue_part.contains("overdue") {
                    *is_overdue = true;
                    // Try to extract number of days
                    if let Some(days_str) = overdue_part
                        .trim_start_matches('(')
                        .split(' ')
                        .next()
                    {
                        if let Ok(days) = days_str.parse::<i32>() {
                            *days_overdue = Some(days);
                        }
                    }
                }
            } else {
                *due_date = Some(format_due_date(date_part));
            }
        } else if !part.is_empty() && part != "-" && account.is_none() {
            // First non-empty part that's not a date is likely the account
            // But skip if it looks like a project reference "(via ...)"
            if !part.starts_with("(via") && !part.starts_with("via") {
                *account = Some(part.to_string());
            } else if let Some(acc) = account.as_mut() {
                // Append project info to account
                acc.push_str(&format!(" {}", part));
            }
        }
    }
}

/// Format a due date for display
fn format_due_date(date_str: &str) -> String {
    // Handle relative dates
    match date_str.to_lowercase().as_str() {
        "today" => "Today".to_string(),
        "tomorrow" => "Tomorrow".to_string(),
        "yesterday" => "Yesterday".to_string(),
        _ => {
            // Try to parse YYYY-MM-DD
            if date_str.len() == 10 && date_str.chars().nth(4) == Some('-') {
                format_date(date_str)
            } else {
                date_str.to_string()
            }
        }
    }
}

/// Inbox directory name
const INBOX_DIR: &str = "_inbox";

/// Count files in the inbox directory
pub fn count_inbox(workspace: &Path) -> usize {
    let inbox_path = workspace.join(INBOX_DIR);
    if !inbox_path.exists() {
        return 0;
    }

    fs::read_dir(&inbox_path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file()
                        && e.path()
                            .extension()
                            .map(|ext| ext == "md")
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// List files in the _inbox/ directory with metadata and preview
/// Determine the high-level file type from an extension.
fn classify_file_type(ext: &str) -> InboxFileType {
    match ext.to_lowercase().as_str() {
        "md" | "markdown" => InboxFileType::Markdown,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "heic" => {
            InboxFileType::Image
        }
        "xlsx" | "xls" | "numbers" | "ods" => InboxFileType::Spreadsheet,
        "docx" | "doc" | "pages" | "odt" | "rtf" | "pdf" => InboxFileType::Document,
        "csv" | "tsv" | "json" | "yaml" | "yml" | "xml" | "toml" => InboxFileType::Data,
        "txt" | "log" | "text" => InboxFileType::Text,
        _ => InboxFileType::Other,
    }
}

/// Return true if the file type supports text preview (readable with `read_to_string`).
fn is_text_previewable(file_type: &InboxFileType) -> bool {
    matches!(
        file_type,
        InboxFileType::Markdown
            | InboxFileType::Data
            | InboxFileType::Text
    )
}

/// Generate a preview for a text-based file (markdown, data, plain text).
fn text_preview(path: &Path, file_type: &InboxFileType) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;

    let mut text = String::new();

    match file_type {
        InboxFileType::Markdown => {
            // Skip frontmatter and blank lines, take first 200 chars of content
            let mut in_frontmatter = false;
            for line in content.lines() {
                if line.trim() == "---" {
                    in_frontmatter = !in_frontmatter;
                    continue;
                }
                if in_frontmatter {
                    continue;
                }
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
                if text.len() >= 200 {
                    break;
                }
            }
        }
        InboxFileType::Data => {
            // For CSV/JSON/YAML: show first few lines verbatim
            for line in content.lines().take(6) {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(line);
                if text.len() >= 200 {
                    break;
                }
            }
        }
        _ => {
            // Plain text: first 200 chars
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
                if text.len() >= 200 {
                    break;
                }
            }
        }
    }

    if text.len() > 200 {
        text.truncate(200);
        text.push_str("...");
    }
    if text.is_empty() { None } else { Some(text) }
}

/// Generate a descriptive preview for binary/non-text files.
fn binary_preview(file_type: &InboxFileType, size_bytes: u64) -> Option<String> {
    let size_label = if size_bytes < 1024 {
        format!("{} B", size_bytes)
    } else if size_bytes < 1024 * 1024 {
        format!("{:.1} KB", size_bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
    };

    let desc = match file_type {
        InboxFileType::Image => format!("Image file — {}", size_label),
        InboxFileType::Spreadsheet => format!("Spreadsheet — {}", size_label),
        InboxFileType::Document => format!("Document — {}", size_label),
        InboxFileType::Other => format!("File — {}", size_label),
        _ => return None, // text types handled by text_preview
    };
    Some(desc)
}

pub fn list_inbox_files(workspace: &Path) -> Vec<InboxFile> {
    let inbox_path = workspace.join(INBOX_DIR);
    if !inbox_path.exists() {
        return Vec::new();
    }

    let mut files: Vec<InboxFile> = fs::read_dir(&inbox_path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            // Include all files, skip hidden files and directories
            path.is_file()
                && !e
                    .file_name()
                    .to_str()
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(true)
        })
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            let filename = entry.file_name().to_str()?.to_string();

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let file_type = classify_file_type(ext);

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                    let dt = chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)?;
                    Some(dt.to_rfc3339())
                })
                .unwrap_or_default();

            let preview = if is_text_previewable(&file_type) {
                text_preview(&path, &file_type)
            } else {
                binary_preview(&file_type, metadata.len())
            };

            Some(InboxFile {
                filename,
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                modified,
                preview,
                file_type,
            })
        })
        .collect();

    // Sort by modified date, newest first
    files.sort_by(|a, b| b.modified.cmp(&a.modified));

    files
}

/// Calculate day stats from parsed data
pub fn calculate_stats(meetings: &[Meeting], actions: &[Action], inbox_count: usize) -> DayStats {
    let total_meetings = meetings.len();
    let customer_meetings = meetings
        .iter()
        .filter(|m| matches!(m.meeting_type, MeetingType::Customer | MeetingType::Qbr))
        .count();
    let actions_due = actions
        .iter()
        .filter(|a| {
            matches!(a.status, ActionStatus::Pending)
                && a.due_date
                    .as_ref()
                    .map(|d| d == "Today" || d.contains("Yesterday"))
                    .unwrap_or(false)
        })
        .count();

    DayStats {
        total_meetings,
        customer_meetings,
        actions_due,
        inbox_count,
    }
}

/// Parse emails from the overview.md table format
/// Format:
/// | From | Subject | Notes |
/// |------|---------|-------|
/// | Sender Name <email | Subject text | 🔴 Customer |
pub fn parse_emails_from_overview(overview_path: &Path) -> Result<Vec<Email>, String> {
    let content = fs::read_to_string(overview_path)
        .map_err(|e| format!("Failed to read overview for emails: {}", e))?;

    let mut emails = Vec::new();
    let mut id_counter = 1;
    let mut in_email_section = false;
    let mut in_table = false;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Detect email section
        if line_trimmed.contains("Email") && line_trimmed.contains("Attention") {
            in_email_section = true;
            continue;
        }

        // Exit on next major section
        if in_email_section && line_trimmed.starts_with("## ") && !line_trimmed.contains("Email") {
            in_email_section = false;
            continue;
        }

        // Parse table rows in email section
        if in_email_section && line_trimmed.starts_with('|') {
            // Skip separator rows
            if line_trimmed.contains("---") {
                in_table = true;
                continue;
            }

            // Skip header row
            if line_trimmed.contains("From") && line_trimmed.contains("Subject") {
                in_table = true;
                continue;
            }

            if !in_table {
                continue;
            }

            // Parse: | From | Subject | Notes |
            let cells: Vec<&str> = line_trimmed.split('|').map(|s| s.trim()).collect();
            if cells.len() >= 4 {
                let from = cells.get(1).unwrap_or(&"").to_string();
                let subject = cells.get(2).unwrap_or(&"").to_string();
                let notes = cells.get(3).unwrap_or(&"").to_string();

                if from.is_empty() || subject.is_empty() {
                    continue;
                }

                // Parse sender: "Name <email" (may be truncated)
                let (sender_name, sender_email) = if let Some(lt) = from.find('<') {
                    let name = from[..lt].trim().to_string();
                    let email = from[lt + 1..].trim_end_matches('>').to_string();
                    (name, email)
                } else {
                    (from.clone(), String::new())
                };

                // Determine priority from notes emoji
                let priority = if notes.contains("🔴") || notes.to_lowercase().contains("customer") {
                    EmailPriority::High
                } else if notes.contains("🟡") || notes.to_lowercase().contains("review") {
                    EmailPriority::Normal
                } else {
                    EmailPriority::Normal
                };

                emails.push(Email {
                    id: format!("e{}", id_counter),
                    sender: sender_name,
                    sender_email,
                    subject,
                    snippet: Some(notes),
                    priority,
                    avatar_url: None,
                });
                id_counter += 1;
            }
        }
    }

    Ok(emails)
}

/// Parse the emails.md file into a list of Email structs
/// Format:
/// ## Emails Needing Attention
/// - **Sender Name** <email@example.com> [high]
///   Subject line here
pub fn parse_emails(path: &Path) -> Result<Vec<Email>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read emails: {}", e))?;

    let mut emails = Vec::new();
    let mut id_counter = 1;
    let mut current_email: Option<EmailBuilder> = None;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Email header: - **Sender Name** <email@example.com> [priority]
        // or: - Sender Name <email@example.com>
        if line_trimmed.starts_with("- ") {
            // Save previous email
            if let Some(builder) = current_email.take() {
                if let Some(email) = builder.build() {
                    emails.push(email);
                }
            }

            if let Some(email) = parse_email_line(line_trimmed, id_counter) {
                current_email = Some(email);
                id_counter += 1;
            }
            continue;
        }

        // Subject line (indented continuation)
        if let Some(ref mut builder) = current_email {
            if !line_trimmed.is_empty() && !line_trimmed.starts_with('#') {
                builder.subject = line_trimmed.to_string();
            }
        }
    }

    // Don't forget the last email
    if let Some(builder) = current_email {
        if let Some(email) = builder.build() {
            emails.push(email);
        }
    }

    Ok(emails)
}

/// Parse a single email line: - **Sender** <email> [priority]
fn parse_email_line(line: &str, id: usize) -> Option<EmailBuilder> {
    let rest = line.strip_prefix("- ")?.trim();

    // Extract sender name (may be in **bold**)
    let (sender, rest) = if rest.starts_with("**") {
        let end = rest[2..].find("**")?;
        let name = rest[2..2 + end].to_string();
        (name, rest[4 + end..].trim())
    } else {
        // No bold, sender ends at <
        let end = rest.find('<')?;
        (rest[..end].trim().to_string(), &rest[end..])
    };

    // Extract email: <email@example.com>
    let email_start = rest.find('<')?;
    let email_end = rest.find('>')?;
    let sender_email = rest[email_start + 1..email_end].to_string();

    // Check for priority tag: [high] or [normal]
    let after_email = &rest[email_end + 1..];
    let priority = if after_email.contains("[high]") {
        EmailPriority::High
    } else {
        EmailPriority::Normal
    };

    Some(EmailBuilder {
        id: format!("e{}", id),
        sender,
        sender_email,
        subject: String::new(),
        priority,
    })
}

struct EmailBuilder {
    id: String,
    sender: String,
    sender_email: String,
    subject: String,
    priority: EmailPriority,
}

impl EmailBuilder {
    fn build(self) -> Option<Email> {
        if self.sender.is_empty() || self.sender_email.is_empty() {
            return None;
        }
        Some(Email {
            id: self.id,
            sender: self.sender,
            sender_email: self.sender_email,
            subject: self.subject,
            snippet: None,
            priority: self.priority,
            avatar_url: None,
        })
    }
}

// =============================================================================
// DailyOS-specific Parsers
// =============================================================================

/// Discover meeting prep files in the _today directory
/// Returns a list of (filename, time, type) tuples for files matching 01-79-*-prep.md
pub fn discover_meeting_preps(today_dir: &Path) -> Vec<(String, String, String)> {
    let mut preps = Vec::new();

    if let Ok(entries) = fs::read_dir(today_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                // Match pattern: NN-HHMM-type-name-prep.md where NN is 01-79
                if filename.ends_with("-prep.md") {
                    let parts: Vec<&str> = filename.split('-').collect();
                    if parts.len() >= 3 {
                        if let Ok(num) = parts[0].parse::<u32>() {
                            if (1..=79).contains(&num) {
                                // Extract time (HHMM) and type
                                let time = if parts.len() > 1 { parts[1] } else { "" };
                                let meeting_type = if parts.len() > 2 { parts[2] } else { "internal" };
                                preps.push((
                                    filename.to_string(),
                                    format_time_from_hhmm(time),
                                    meeting_type.to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by filename (which should sort by time due to format)
    preps.sort_by(|a, b| a.0.cmp(&b.0));
    preps
}

/// Format HHMM to display time (e.g., "1630" -> "4:30 PM")
fn format_time_from_hhmm(hhmm: &str) -> String {
    if hhmm.len() != 4 {
        return hhmm.to_string();
    }

    let hours: u32 = hhmm[..2].parse().unwrap_or(0);
    let minutes: u32 = hhmm[2..].parse().unwrap_or(0);

    let (display_hour, period) = if hours == 0 {
        (12, "AM")
    } else if hours < 12 {
        (hours, "AM")
    } else if hours == 12 {
        (12, "PM")
    } else {
        (hours - 12, "PM")
    };

    if minutes == 0 {
        format!("{}:00 {}", display_hour, period)
    } else {
        format!("{}:{:02} {}", display_hour, minutes, period)
    }
}

/// Parse a full meeting prep file (01-HHMM-type-name-prep.md)
pub fn parse_meeting_prep_file(path: &Path) -> Result<FullMeetingPrep, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read prep file: {}", e))?;

    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut title = String::new();
    let mut time_range = String::new();
    let mut meeting_context: Option<String> = None;
    let mut attendees: Vec<Stakeholder> = Vec::new();
    let mut current_state: Vec<String> = Vec::new();
    let mut open_items: Vec<ActionWithContext> = Vec::new();
    let mut questions: Vec<String> = Vec::new();
    let mut key_principles: Vec<String> = Vec::new();
    let mut references: Vec<SourceReference> = Vec::new();

    // Dedicated fields for DailyOS format sections
    let mut quick_context: Vec<(String, String)> = Vec::new();
    let mut since_last: Vec<String> = Vec::new();
    let mut strategic_programs: Vec<String> = Vec::new();
    let mut risks: Vec<String> = Vec::new();
    let mut talking_points: Vec<String> = Vec::new();

    let mut current_section = String::new();
    let mut in_table = false;
    let mut in_frontmatter = false;
    let mut pending_context_paragraphs: Vec<String> = Vec::new();

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Handle frontmatter
        if line_trimmed == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }
        if in_frontmatter {
            continue;
        }

        // Main title: # Title
        if line_trimmed.starts_with("# ") && title.is_empty() {
            title = line_trimmed.strip_prefix("# ").unwrap_or("").to_string();
            continue;
        }

        // Time range: **TIME - TIME | Title**
        if line_trimmed.starts_with("**") && line_trimmed.contains("|") && time_range.is_empty() {
            if let Some(time_part) = line_trimmed.strip_prefix("**") {
                if let Some(pipe_pos) = time_part.find('|') {
                    time_range = time_part[..pipe_pos].trim().trim_end_matches("**").to_string();
                }
            }
            continue;
        }

        // Section headers
        if line_trimmed.starts_with("## ") {
            // Save pending context paragraphs before moving to new section
            if !pending_context_paragraphs.is_empty() && meeting_context.is_none() {
                meeting_context = Some(pending_context_paragraphs.join("\n\n"));
                pending_context_paragraphs.clear();
            }

            current_section = line_trimmed.strip_prefix("## ").unwrap_or("").to_lowercase();
            in_table = false;
            continue;
        }
        if line_trimmed.starts_with("### ") {
            current_section = line_trimmed.strip_prefix("### ").unwrap_or("").to_lowercase();
            in_table = false;
            continue;
        }

        // Table detection and parsing
        if line_trimmed.starts_with('|') {
            // Skip separator rows
            if line_trimmed.contains("---") {
                in_table = true;
                continue;
            }

            // Skip header rows
            if line_trimmed.to_lowercase().contains("metric") && line_trimmed.to_lowercase().contains("value") {
                in_table = true;
                continue;
            }
            if line_trimmed.contains("Name") && line_trimmed.contains("Role") {
                in_table = true;
                continue;
            }
            if line_trimmed.contains("Document") && line_trimmed.contains("Path") {
                in_table = true;
                continue;
            }

            in_table = true;

            // Parse Quick Context table (Metric | Value format)
            if current_section.contains("quick context") {
                let cells: Vec<&str> = line_trimmed.split('|').map(|s| s.trim()).collect();
                if cells.len() >= 3 {
                    let metric = cells.get(1).unwrap_or(&"").trim_start_matches("**").trim_end_matches("**").to_string();
                    let value = cells.get(2).unwrap_or(&"").to_string();
                    if !metric.is_empty() && !value.is_empty() && !metric.to_lowercase().contains("metric") {
                        quick_context.push((metric, value));
                    }
                }
            }

            // Parse attendees table
            if current_section.contains("attendee") || current_section.contains("key stakeholder") {
                let cells: Vec<&str> = line_trimmed.split('|').collect();
                if cells.len() >= 3 {
                    let name = cells.get(1).map(|s| s.trim().trim_start_matches("**").trim_end_matches("**")).unwrap_or("");
                    let role = cells.get(2).map(|s| s.trim());
                    let focus = cells.get(3).map(|s| s.trim());

                    if !name.is_empty() && !name.to_lowercase().contains("name") {
                        attendees.push(Stakeholder {
                            name: name.to_string(),
                            role: role.filter(|s| !s.is_empty()).map(String::from),
                            focus: focus.filter(|s| !s.is_empty()).map(String::from),
                        });
                    }
                }
            }

            // Parse references table
            if current_section.contains("reference") {
                let cells: Vec<&str> = line_trimmed.split('|').collect();
                if cells.len() >= 3 {
                    let label = cells.get(1).map(|s| s.trim()).unwrap_or("");
                    let path_val = cells.get(2).map(|s| s.trim().trim_matches('`'));

                    if !label.is_empty() && !label.to_lowercase().contains("document") {
                        references.push(SourceReference {
                            label: label.to_string(),
                            path: path_val.filter(|s| !s.is_empty()).map(String::from),
                            last_updated: None,
                        });
                    }
                }
            }
            continue;
        }

        // List items with checkboxes: - [x] or - [ ]
        if line_trimmed.starts_with("- [") {
            let is_checked = line_trimmed.contains("[x]") || line_trimmed.contains("[X]");
            let item_text = line_trimmed
                .strip_prefix("- [x] ")
                .or_else(|| line_trimmed.strip_prefix("- [X] "))
                .or_else(|| line_trimmed.strip_prefix("- [ ] "))
                .unwrap_or(&line_trimmed[5..])
                .to_string();

            if current_section.contains("strategic program") || current_section.contains("current strategic") {
                let status = if is_checked { "✓ " } else { "○ " };
                strategic_programs.push(format!("{}{}", status, item_text));
            } else if current_section.contains("open action") || current_section.contains("action item") {
                open_items.push(ActionWithContext {
                    title: item_text,
                    due_date: None,
                    context: None,
                    is_overdue: current_section.contains("overdue"),
                });
            }
            continue;
        }

        // Regular list items: - or *
        if line_trimmed.starts_with("- ") || line_trimmed.starts_with("* ") {
            let item = line_trimmed[2..].to_string();

            if current_section.contains("question") || current_section.contains("ask") {
                questions.push(item);
            } else if current_section.contains("since last") {
                since_last.push(item);
            } else if current_section.contains("risk") {
                risks.push(item);
            } else if current_section.contains("talking point") || current_section.contains("suggested talking") {
                talking_points.push(item);
            } else if current_section.contains("current state") || current_section.contains("product track") || current_section.contains("partnership track") {
                current_state.push(item);
            } else if current_section.contains("open item") || current_section.contains("overdue") || current_section.contains("action") {
                let is_overdue = current_section.contains("overdue");
                open_items.push(ActionWithContext {
                    title: item,
                    due_date: None,
                    context: None,
                    is_overdue,
                });
            } else if current_section.contains("principle") {
                key_principles.push(item);
            }
            continue;
        }

        // Numbered list items: 1. 2. etc
        if line_trimmed.len() > 2 && line_trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            if let Some(dot_pos) = line_trimmed.find(". ") {
                let item = line_trimmed[dot_pos + 2..].to_string();

                if current_section.contains("talking point") || current_section.contains("suggested talking") {
                    talking_points.push(item);
                } else if current_section.contains("question") || current_section.contains("ask") {
                    questions.push(item);
                }
                continue;
            }
        }

        // Blockquote for principles
        if line_trimmed.starts_with('>') && current_section.contains("principle") {
            let quote = line_trimmed.strip_prefix('>').unwrap_or("").trim();
            if !quote.is_empty() {
                key_principles.push(quote.to_string());
            }
            continue;
        }

        // Bold inline notes: **Note**: text
        if line_trimmed.starts_with("**Note") {
            if current_section.contains("context") {
                pending_context_paragraphs.push(line_trimmed.to_string());
            }
            continue;
        }

        // General context paragraph (non-table text in context sections)
        if current_section.contains("meeting title context") && !line_trimmed.is_empty() && !in_table {
            pending_context_paragraphs.push(line_trimmed.to_string());
        }
    }

    // Finalize meeting context from paragraphs
    if meeting_context.is_none() && !pending_context_paragraphs.is_empty() {
        meeting_context = Some(pending_context_paragraphs.join("\n\n"));
    }

    Ok(FullMeetingPrep {
        file_path: filename,
        title,
        time_range,
        meeting_context,
        quick_context: if quick_context.is_empty() { None } else { Some(quick_context) },
        attendees: if attendees.is_empty() { None } else { Some(attendees) },
        since_last: if since_last.is_empty() { None } else { Some(since_last) },
        strategic_programs: if strategic_programs.is_empty() { None } else { Some(strategic_programs) },
        current_state: if current_state.is_empty() { None } else { Some(current_state) },
        open_items: if open_items.is_empty() { None } else { Some(open_items) },
        risks: if risks.is_empty() { None } else { Some(risks) },
        talking_points: if talking_points.is_empty() { None } else { Some(talking_points) },
        questions: if questions.is_empty() { None } else { Some(questions) },
        key_principles: if key_principles.is_empty() { None } else { Some(key_principles) },
        references: if references.is_empty() { None } else { Some(references) },
        raw_markdown: Some(content),
    })
}

/// Parse the 83-email-summary.md file into EmailSummaryData
pub fn parse_email_summary(path: &Path) -> Result<EmailSummaryData, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read email summary: {}", e))?;

    let mut high_priority: Vec<EmailDetail> = Vec::new();
    let mut medium_priority: Vec<EmailDetail> = Vec::new();
    let mut current_email: Option<EmailDetailBuilder> = None;
    let mut id_counter = 1;
    let mut in_high = false;
    let mut in_medium = false;
    let mut stats = EmailStats {
        high_count: 0,
        medium_count: 0,
        low_count: 0,
        needs_action: None,
    };

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Section detection
        if line_trimmed.contains("HIGH Priority") {
            in_high = true;
            in_medium = false;
            continue;
        }
        if line_trimmed.contains("Medium Priority") || line_trimmed.contains("Notable Medium") {
            // Save current email
            if let Some(builder) = current_email.take() {
                if in_high {
                    high_priority.push(builder.build(id_counter));
                } else {
                    medium_priority.push(builder.build(id_counter));
                }
                id_counter += 1;
            }
            in_high = false;
            in_medium = true;
            continue;
        }
        if line_trimmed.starts_with("## Summary") || line_trimmed.starts_with("---") {
            // Save current email
            if let Some(builder) = current_email.take() {
                if in_high {
                    high_priority.push(builder.build(id_counter));
                } else if in_medium {
                    medium_priority.push(builder.build(id_counter));
                }
                id_counter += 1;
            }
            in_high = false;
            in_medium = false;
            continue;
        }

        // Email entry: ### N. Sender Name (Account) - Subject Context
        if line_trimmed.starts_with("### ") {
            // Save previous email
            if let Some(builder) = current_email.take() {
                if in_high {
                    high_priority.push(builder.build(id_counter));
                } else if in_medium {
                    medium_priority.push(builder.build(id_counter));
                }
                id_counter += 1;
            }

            let header = line_trimmed.strip_prefix("### ").unwrap_or("");
            // Skip the number prefix (e.g., "1. ")
            let header = if let Some(dot_pos) = header.find(". ") {
                &header[dot_pos + 2..]
            } else {
                header
            };

            current_email = Some(EmailDetailBuilder {
                sender: header.to_string(),
                sender_email: String::new(),
                subject: String::new(),
                received: None,
                priority: if in_high { EmailPriority::High } else { EmailPriority::Normal },
                email_type: None,
                summary: None,
                conversation_arc: None,
                recommended_action: None,
                action_owner: None,
                action_priority: None,
            });
            continue;
        }

        // Parse table rows for email details
        if let Some(ref mut builder) = current_email {
            if line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
                let cells: Vec<&str> = line_trimmed.split('|').map(|s| s.trim()).collect();
                if cells.len() >= 3 {
                    let field = cells.get(1).unwrap_or(&"");
                    let value = cells.get(2).unwrap_or(&"").trim_start_matches("**").trim_end_matches("**");

                    match field.to_lowercase().as_str() {
                        "from" | "**from**" => {
                            // Parse "Name <email@example.com>"
                            if let Some(lt) = value.find('<') {
                                builder.sender = value[..lt].trim().to_string();
                                if let Some(gt) = value.find('>') {
                                    builder.sender_email = value[lt + 1..gt].to_string();
                                }
                            }
                        }
                        "subject" | "**subject**" => {
                            builder.subject = value.to_string();
                        }
                        "received" | "**received**" => {
                            builder.received = Some(value.to_string());
                        }
                        "type" | "**type**" => {
                            builder.email_type = Some(value.to_string());
                        }
                        _ => {}
                    }
                }
                continue;
            }

            // Summary, conversation arc, action
            if line_trimmed.starts_with("**Summary:**") || line_trimmed.starts_with("**Summary**:") {
                builder.summary = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            } else if line_trimmed.starts_with("**Conversation Arc:**") || line_trimmed.starts_with("**Conversation Arc**:") {
                builder.conversation_arc = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            } else if line_trimmed.starts_with("**Action for") || line_trimmed.starts_with("**Specific Ask") {
                builder.recommended_action = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            } else if line_trimmed.starts_with("**Recommended Action:**") || line_trimmed.starts_with("**Recommended Action**:") {
                builder.recommended_action = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            } else if line_trimmed.starts_with("**Owner:**") || line_trimmed.starts_with("**Owner**:") {
                builder.action_owner = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            } else if line_trimmed.starts_with("**Priority:**") || line_trimmed.starts_with("**Priority**:") {
                builder.action_priority = Some(line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string());
            }
        }

        // Parse stats from summary section
        if line_trimmed.starts_with("**High Priority**:") || line_trimmed.starts_with("| **HIGH Priority**") {
            if let Some(count) = extract_count_from_line(line_trimmed) {
                stats.high_count = count;
            }
        } else if line_trimmed.starts_with("**Medium**:") || line_trimmed.starts_with("| **Medium**") {
            if let Some(count) = extract_count_from_line(line_trimmed) {
                stats.medium_count = count;
            }
        } else if line_trimmed.starts_with("**Low") || line_trimmed.starts_with("| **Low") {
            if let Some(count) = extract_count_from_line(line_trimmed) {
                stats.low_count = count;
            }
        }
    }

    // Don't forget the last email
    if let Some(builder) = current_email {
        if in_high {
            high_priority.push(builder.build(id_counter));
        } else if in_medium {
            medium_priority.push(builder.build(id_counter));
        }
    }

    // Update stats from actual counts if not parsed
    if stats.high_count == 0 {
        stats.high_count = high_priority.len();
    }

    Ok(EmailSummaryData {
        high_priority,
        medium_priority: if medium_priority.is_empty() { None } else { Some(medium_priority) },
        stats,
    })
}

/// Extract a count number from a stats line
fn extract_count_from_line(line: &str) -> Option<usize> {
    // Look for patterns like ": 2" or "| 2 |"
    for word in line.split(|c: char| !c.is_ascii_digit()) {
        if let Ok(n) = word.parse::<usize>() {
            return Some(n);
        }
    }
    None
}

struct EmailDetailBuilder {
    sender: String,
    sender_email: String,
    subject: String,
    received: Option<String>,
    priority: EmailPriority,
    email_type: Option<String>,
    summary: Option<String>,
    conversation_arc: Option<String>,
    recommended_action: Option<String>,
    action_owner: Option<String>,
    action_priority: Option<String>,
}

impl EmailDetailBuilder {
    fn build(self, id: usize) -> EmailDetail {
        EmailDetail {
            id: format!("ed{}", id),
            sender: self.sender,
            sender_email: self.sender_email,
            subject: self.subject,
            received: self.received,
            priority: self.priority,
            email_type: self.email_type,
            summary: self.summary,
            conversation_arc: self.conversation_arc,
            recommended_action: self.recommended_action,
            action_owner: self.action_owner,
            action_priority: self.action_priority,
        }
    }
}

/// Parse the 81-suggested-focus.md file into FocusData
pub fn parse_focus(path: &Path) -> Result<FocusData, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read focus file: {}", e))?;

    let mut priorities: Vec<FocusPriority> = Vec::new();
    let mut time_blocks: Vec<TimeBlock> = Vec::new();
    let mut quick_wins: Vec<String> = Vec::new();
    let mut energy_notes = EnergyNotes {
        morning: None,
        afternoon: None,
    };

    let mut current_section = String::new();
    let mut current_priority: Option<FocusPriority> = None;

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Section headers
        if line_trimmed.starts_with("## ") {
            // Save current priority
            if let Some(p) = current_priority.take() {
                priorities.push(p);
            }

            current_section = line_trimmed.strip_prefix("## ").unwrap_or("").to_lowercase();

            // Check for priority sections
            if current_section.starts_with("priority") {
                let parts: Vec<&str> = current_section.split(':').collect();
                let level = parts.get(0).unwrap_or(&"").trim().to_string();
                let label = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
                current_priority = Some(FocusPriority {
                    level,
                    label,
                    items: Vec::new(),
                });
            }
            continue;
        }

        // List items
        if line_trimmed.starts_with("- ") {
            let item = line_trimmed.strip_prefix("- ").unwrap_or("").to_string();
            // Remove checkbox if present
            let item = item.strip_prefix("[ ] ").unwrap_or(&item).to_string();
            let item = item.strip_prefix("[x] ").unwrap_or(&item).to_string();

            if let Some(ref mut p) = current_priority {
                p.items.push(item);
            } else if current_section.contains("quick win") || current_section.contains("downtime") {
                quick_wins.push(item);
            } else if current_section.contains("time block") || current_section.contains("available") {
                // Parse time block: "09:30 - 12:00 (150 min available)"
                if let Some(block) = parse_time_block_line(&item) {
                    time_blocks.push(block);
                }
            }
            continue;
        }

        // Energy notes
        if current_section.contains("energy") {
            if line_trimmed.starts_with("**Morning") || line_trimmed.to_lowercase().contains("morning") {
                let note = line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string();
                if !note.is_empty() {
                    energy_notes.morning = Some(note);
                }
            } else if line_trimmed.starts_with("**Afternoon") || line_trimmed.to_lowercase().contains("afternoon") {
                let note = line_trimmed.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string();
                if !note.is_empty() {
                    energy_notes.afternoon = Some(note);
                }
            }
        }
    }

    // Don't forget the last priority
    if let Some(p) = current_priority {
        priorities.push(p);
    }

    Ok(FocusData {
        priorities,
        time_blocks: if time_blocks.is_empty() { None } else { Some(time_blocks) },
        quick_wins: if quick_wins.is_empty() { None } else { Some(quick_wins) },
        energy_notes: if energy_notes.morning.is_some() || energy_notes.afternoon.is_some() {
            Some(energy_notes)
        } else {
            None
        },
    })
}

/// Parse a time block line like "09:30 - 12:00 (150 min available)"
fn parse_time_block_line(line: &str) -> Option<TimeBlock> {
    // Pattern: HH:MM - HH:MM (duration)
    let parts: Vec<&str> = line.split(" - ").collect();
    if parts.len() < 2 {
        return None;
    }

    let start = parts[0].trim();
    let rest = parts[1..].join(" - ");

    // Extract end time and duration
    let (end, duration_str) = if let Some(paren_pos) = rest.find('(') {
        (rest[..paren_pos].trim(), &rest[paren_pos..])
    } else {
        (rest.trim(), "")
    };

    // Parse duration
    let duration_minutes = duration_str
        .trim_matches(|c| c == '(' || c == ')')
        .split(|c: char| !c.is_ascii_digit())
        .find_map(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    Some(TimeBlock {
        day: String::new(), // Will be filled in by caller if needed
        start: start.to_string(),
        end: end.to_string(),
        duration_minutes,
        suggested_use: None,
    })
}

/// Parse the week-00-overview.md file into WeekOverview
pub fn parse_week_overview(path: &Path) -> Result<WeekOverview, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read week overview: {}", e))?;

    let mut week_number = String::new();
    let mut date_range = String::new();
    let mut days: Vec<WeekDay> = Vec::new();
    let mut action_summary: Option<WeekActionSummary> = None;
    let mut hygiene_alerts: Vec<HygieneAlert> = Vec::new();
    let mut focus_areas: Vec<String> = Vec::new();
    let mut available_time_blocks: Vec<TimeBlock> = Vec::new();

    let mut current_section = String::new();
    let mut in_meetings_table = false;
    let mut in_hygiene_table = false;
    let mut in_time_table = false;
    let mut overdue_count: usize = 0;
    let mut due_this_week: usize = 0;
    let mut critical_items: Vec<String> = Vec::new();

    for line in content.lines() {
        let line_trimmed = line.trim();

        // Title: # Week Overview: W06 - February 02-06, 2026
        if line_trimmed.starts_with("# Week") {
            if let Some(rest) = line_trimmed.strip_prefix("# Week Overview:") {
                let rest = rest.trim();
                // Extract W06 and date range
                let parts: Vec<&str> = rest.split(" - ").collect();
                if !parts.is_empty() {
                    week_number = parts[0].trim().to_string();
                }
                if parts.len() > 1 {
                    date_range = parts[1..].join(" - ");
                }
            }
            continue;
        }

        // Section headers
        if line_trimmed.starts_with("## ") {
            current_section = line_trimmed.strip_prefix("## ").unwrap_or("").to_lowercase();
            in_meetings_table = current_section.contains("meeting");
            in_hygiene_table = current_section.contains("hygiene");
            in_time_table = current_section.contains("available time") || current_section.contains("time block");
            continue;
        }
        if line_trimmed.starts_with("### ") {
            let subsection = line_trimmed.strip_prefix("### ").unwrap_or("").to_lowercase();
            if subsection.contains("overdue") {
                // Try to extract count: "### Overdue (18)"
                if let Some(count) = extract_count_from_line(line_trimmed) {
                    overdue_count = count;
                }
            } else if subsection.contains("due this week") {
                if let Some(count) = extract_count_from_line(line_trimmed) {
                    due_this_week = count;
                }
            } else if subsection.contains("critical") {
                // Track that we're in critical items section
                current_section = "critical".to_string();
            }
            continue;
        }

        // Meetings table
        if in_meetings_table && line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
            if !line_trimmed.contains("Day") && !line_trimmed.contains("Time") {
                if let Some(meeting) = parse_week_meeting_row(line_trimmed) {
                    // Group by day
                    let day_name = meeting.0.clone();
                    if let Some(day) = days.iter_mut().find(|d| d.day_name == day_name) {
                        day.meetings.push(meeting.1);
                    } else {
                        days.push(WeekDay {
                            date: String::new(),
                            day_name: day_name.clone(),
                            meetings: vec![meeting.1],
                        });
                    }
                }
            }
            continue;
        }

        // Hygiene alerts table
        if in_hygiene_table && line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
            if !line_trimmed.contains("Account") && !line_trimmed.contains("Ring") {
                if let Some(alert) = parse_hygiene_alert_row(line_trimmed) {
                    hygiene_alerts.push(alert);
                }
            }
            continue;
        }

        // Available time blocks table
        if in_time_table && line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
            if !line_trimmed.contains("Day") && !line_trimmed.contains("Time") {
                if let Some(block) = parse_time_block_row(line_trimmed) {
                    available_time_blocks.push(block);
                }
            }
            continue;
        }

        // Critical items (list)
        if current_section == "critical" && line_trimmed.starts_with("- ") {
            let item = line_trimmed.strip_prefix("- ").unwrap_or("");
            let item = item.strip_prefix("[ ] ").unwrap_or(item);
            let item = item.strip_prefix("**").unwrap_or(item);
            if let Some(end) = item.find("**") {
                critical_items.push(item[..end].to_string());
            } else {
                critical_items.push(item.to_string());
            }
            continue;
        }

        // Focus areas (list)
        if current_section.contains("priorit") || current_section.contains("focus") {
            if line_trimmed.starts_with("1.") || line_trimmed.starts_with("- ") {
                let item = line_trimmed.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == ' ');
                if let Some(colon) = item.find(':') {
                    focus_areas.push(item[..colon].trim_start_matches("**").trim_end_matches("**").to_string());
                } else {
                    focus_areas.push(item.to_string());
                }
            }
        }
    }

    // Build action summary
    if overdue_count > 0 || due_this_week > 0 || !critical_items.is_empty() {
        action_summary = Some(WeekActionSummary {
            overdue_count,
            due_this_week,
            critical_items,
        });
    }

    Ok(WeekOverview {
        week_number,
        date_range,
        days,
        action_summary,
        hygiene_alerts: if hygiene_alerts.is_empty() { None } else { Some(hygiene_alerts) },
        focus_areas: if focus_areas.is_empty() { None } else { Some(focus_areas) },
        available_time_blocks: if available_time_blocks.is_empty() { None } else { Some(available_time_blocks) },
    })
}

/// Parse a week meeting table row
fn parse_week_meeting_row(line: &str) -> Option<(String, WeekMeeting)> {
    let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if cells.len() < 5 {
        return None;
    }

    let day = cells.get(1)?.to_string();
    let time = cells.get(2)?.to_string();
    let account_or_title = cells.get(3)?.to_string();
    let ring_or_type = cells.get(4).map(|s| s.to_string());
    let prep_status_str = cells.get(5).map(|s| s.to_string());

    // Determine meeting type from ring or explicit type
    let meeting_type = if let Some(ref t) = ring_or_type {
        match t.to_lowercase().as_str() {
            "customer" | "summit" | "foundation" | "evolution" | "influence" => MeetingType::Customer,
            "qbr" => MeetingType::Qbr,
            "training" => MeetingType::Training,
            "internal" => MeetingType::Internal,
            "team_sync" | "team-sync" => MeetingType::TeamSync,
            "one_on_one" | "one-on-one" | "1:1" => MeetingType::OneOnOne,
            "partnership" => MeetingType::Partnership,
            "all_hands" | "all-hands" => MeetingType::AllHands,
            "external" | "project" => MeetingType::External,
            "personal" => MeetingType::Personal,
            _ => MeetingType::Internal,
        }
    } else {
        MeetingType::Internal
    };

    // Parse prep status
    let prep_status = parse_prep_status(prep_status_str.as_deref().unwrap_or(""));

    Some((day, WeekMeeting {
        time,
        title: account_or_title.clone(),
        account: Some(account_or_title),
        meeting_type,
        prep_status,
    }))
}

/// Parse prep status from status string
fn parse_prep_status(status: &str) -> PrepStatus {
    let status_lower = status.to_lowercase();
    if status_lower.contains("prep needed") || status.contains("📋") {
        PrepStatus::PrepNeeded
    } else if status_lower.contains("agenda needed") || status.contains("📅") {
        PrepStatus::AgendaNeeded
    } else if status_lower.contains("bring updates") || status.contains("🔄") {
        PrepStatus::BringUpdates
    } else if status_lower.contains("context needed") || status.contains("👥") {
        PrepStatus::ContextNeeded
    } else if status_lower.contains("prep ready") || status_lower.contains("✅ prep") {
        PrepStatus::PrepReady
    } else if status_lower.contains("draft ready") || status.contains("✏️") {
        PrepStatus::DraftReady
    } else if status_lower.contains("done") || status_lower.contains("✅ done") {
        PrepStatus::Done
    } else {
        PrepStatus::ContextNeeded
    }
}

/// Parse a hygiene alert table row
fn parse_hygiene_alert_row(line: &str) -> Option<HygieneAlert> {
    let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if cells.len() < 4 {
        return None;
    }

    let account = cells.get(1)?.to_string();
    let ring = cells.get(2).map(|s| s.to_string());
    let arr = cells.get(3).map(|s| s.to_string());
    let issue = cells.get(4).map(|s| s.to_string()).unwrap_or_default();

    // Determine severity from section header or issue content
    let severity = if issue.to_lowercase().contains("missing") || issue.to_lowercase().contains("no ") {
        AlertSeverity::Critical
    } else {
        AlertSeverity::Warning
    };

    Some(HygieneAlert {
        account,
        ring: ring.filter(|s| !s.is_empty()),
        arr: arr.filter(|s| !s.is_empty()),
        issue,
        severity,
    })
}

/// Parse a time block table row
fn parse_time_block_row(line: &str) -> Option<TimeBlock> {
    let cells: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if cells.len() < 4 {
        return None;
    }

    let day = cells.get(1)?.to_string();
    let time_range = cells.get(2)?.to_string();
    let duration_str = cells.get(3)?.to_string();
    let suggested_use = cells.get(4).map(|s| s.to_string());

    // Parse time range "9:00 AM - 4:00 PM"
    let time_parts: Vec<&str> = time_range.split(" - ").collect();
    let start = time_parts.get(0).unwrap_or(&"").to_string();
    let end = time_parts.get(1).unwrap_or(&"").to_string();

    // Parse duration
    let duration_minutes = duration_str
        .split(|c: char| !c.is_ascii_digit())
        .find_map(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    Some(TimeBlock {
        day,
        start,
        end,
        duration_minutes,
        suggested_use: suggested_use.filter(|s| !s.is_empty()),
    })
}

/// Parse meetings from 00-overview.md schedule table and match with prep files
pub fn parse_meetings_from_overview(
    overview_path: &Path,
    today_dir: &Path,
) -> Result<Vec<Meeting>, String> {
    let content = fs::read_to_string(overview_path)
        .map_err(|e| format!("Failed to read overview: {}", e))?;

    let mut meetings = Vec::new();
    let mut id_counter = 1;
    let mut in_schedule = false;

    // Discover available prep files
    let prep_files = discover_meeting_preps(today_dir);

    for line in content.lines() {
        let line_trimmed = line.trim();

        if line_trimmed.starts_with("## Schedule") {
            in_schedule = true;
            continue;
        }

        if in_schedule && line_trimmed.starts_with("## ") {
            in_schedule = false;
            continue;
        }

        if in_schedule && line_trimmed.starts_with('|') && !line_trimmed.contains("---") {
            // Skip header row
            if line_trimmed.contains("Time") && line_trimmed.contains("Event") {
                continue;
            }

            // Parse schedule table row: | Time | Event | Type | Prep Status |
            let cells: Vec<&str> = line_trimmed.split('|').map(|s| s.trim()).collect();
            if cells.len() >= 4 {
                let time = cells.get(1).unwrap_or(&"").to_string();
                let event = cells.get(2).unwrap_or(&"").to_string();
                let meeting_type_str = cells.get(3).unwrap_or(&"").to_string();

                if time.is_empty() || event.is_empty() {
                    continue;
                }

                // Determine meeting type
                let meeting_type = parse_meeting_type(&meeting_type_str);

                // Try to find matching prep file by time
                let (prep_file, has_prep, prep_summary) = prep_files
                    .iter()
                    .find(|(_, t, _)| time_matches(&time, t))
                    .map(|(f, _, _)| {
                        let prep_path = today_dir.join(f);
                        // Try to parse prep file and extract summary for dropdown
                        let summary = parse_meeting_prep_file(&prep_path)
                            .ok()
                            .map(|full_prep| extract_prep_summary(&full_prep));
                        (Some(f.clone()), true, summary)
                    })
                    .unwrap_or((None, false, None));

                meetings.push(Meeting {
                    id: format!("m{}", id_counter),
                    time: time.clone(),
                    end_time: None,
                    title: event,
                    meeting_type,
                    account: None, // Would need to parse from event title
                    prep: prep_summary,
                    is_current: None,
                    prep_file,
                    has_prep,
                });

                id_counter += 1;
            }
        }
    }

    Ok(meetings)
}

/// Extract a summary from a full meeting prep for the inline dropdown display
fn extract_prep_summary(full_prep: &FullMeetingPrep) -> MeetingPrep {
    // Build "At a Glance" metrics from various sources:
    // 1. Quick context metrics (customer meetings)
    // 2. Current state items (internal meetings)
    // 3. Since last updates
    let metrics: Option<Vec<String>> = full_prep.quick_context.as_ref()
        .map(|qc| qc.iter().map(|(k, v)| format!("{}: {}", k, v)).collect())
        .or_else(|| full_prep.since_last.clone())
        .or_else(|| full_prep.current_state.clone());

    // Build "Watch" from risks (any meeting type can have blockers/concerns)
    let risks = full_prep.risks.clone();

    // Build "Wins" from completed strategic programs or explicit wins
    let wins: Option<Vec<String>> = full_prep.strategic_programs.as_ref()
        .map(|programs| {
            programs.iter()
                .filter(|s| s.starts_with("✓"))
                .map(|s| s.trim_start_matches("✓ ").to_string())
                .collect()
        })
        .filter(|v: &Vec<String>| !v.is_empty());

    // Build "Discuss" from talking points, questions, or open items (whatever's available)
    let actions: Option<Vec<String>> = full_prep.talking_points.clone()
        .or_else(|| full_prep.questions.clone())
        .or_else(|| full_prep.open_items.as_ref().map(|items| {
            items.iter().map(|i| i.title.clone()).collect()
        }));

    // Build context - prefer meeting_context, fall back to formatted quick_context
    let context: Option<String> = full_prep.meeting_context.clone().or_else(|| {
        full_prep.quick_context.as_ref().map(|qc| {
            qc.iter()
                .take(3)
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(" | ")
        })
    });

    // Keep questions separate as fallback for Discuss
    let questions = if full_prep.talking_points.is_some() {
        full_prep.questions.clone()
    } else {
        None // Already used in actions
    };

    // Extract stakeholders
    let stakeholders = full_prep.attendees.clone();

    // Extract open items as strings
    let open_items: Option<Vec<String>> = full_prep.open_items.as_ref().map(|items| {
        items.iter().map(|i| i.title.clone()).collect()
    });

    // Extract source references
    let source_references = full_prep.references.clone();

    MeetingPrep {
        context,
        metrics,
        risks,
        wins,
        actions,
        stakeholders,
        questions,
        open_items,
        historical_context: None,
        source_references,
    }
}

/// Convert display time to HHMM format
fn time_to_hhmm(time: &str) -> String {
    // Parse "1:00 PM" or "4:30 PM" to "1300" or "1630"
    let time = time.trim().to_uppercase();
    let is_pm = time.contains("PM");
    let time = time.replace("AM", "").replace("PM", "").trim().to_string();

    let parts: Vec<&str> = time.split(':').collect();
    if parts.len() != 2 {
        return String::new();
    }

    let mut hours: u32 = parts[0].trim().parse().unwrap_or(0);
    let minutes: u32 = parts[1].trim().parse().unwrap_or(0);

    if is_pm && hours != 12 {
        hours += 12;
    } else if !is_pm && hours == 12 {
        hours = 0;
    }

    format!("{:02}{:02}", hours, minutes)
}

/// Check if two times match (approximately)
fn time_matches(display_time: &str, parsed_time: &str) -> bool {
    let hhmm = time_to_hhmm(display_time);
    let parsed_hhmm = time_to_hhmm(parsed_time);

    // Exact match
    if hhmm == parsed_hhmm {
        return true;
    }

    // Close match (within 30 minutes)
    if let (Ok(h1), Ok(h2)) = (
        hhmm.parse::<i32>(),
        parsed_hhmm.parse::<i32>(),
    ) {
        let diff = (h1 - h2).abs();
        return diff <= 30 || diff >= 2330; // Handle midnight wrap
    }

    false
}
