use std::fs;
use std::path::Path;

use crate::types::{
    Action, ActionStatus, DayOverview, DayStats, Meeting, MeetingPrep, MeetingType, Priority,
};

/// Parse the overview.md file into a DayOverview struct
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

    for line in content.lines() {
        let line = line.trim();

        // Handle YAML frontmatter
        if line == "---" {
            in_frontmatter = !in_frontmatter;
            continue;
        }

        if in_frontmatter {
            if let Some(value) = line.strip_prefix("date:") {
                date = format_date(value.trim());
            } else if let Some(value) = line.strip_prefix("greeting:") {
                greeting = value.trim().to_string();
            }
            continue;
        }

        // Handle sections
        if line.starts_with("## Focus") || line.starts_with("## focus") {
            in_focus = true;
            in_summary = false;
            continue;
        }

        if line.starts_with("# ") || line.starts_with("## ") {
            in_focus = false;
            in_summary = line.contains("Overview") || line.contains("Summary");
            continue;
        }

        // Capture content
        if in_focus && !line.is_empty() {
            focus = Some(line.to_string());
            in_focus = false;
        } else if in_summary && !line.is_empty() {
            if summary.is_empty() {
                summary = line.to_string();
            } else {
                summary.push(' ');
                summary.push_str(line);
            }
        } else if !line.is_empty() && !line.starts_with('#') && summary.is_empty() {
            // First non-empty line after frontmatter becomes summary if no section found
            summary = line.to_string();
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
        "internal" => MeetingType::Internal,
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
        }
    }

    fn build(self) -> Option<Meeting> {
        Some(Meeting {
            id: self.id,
            time: self.time,
            end_time: self.end_time,
            title: self.title,
            meeting_type: self.meeting_type.unwrap_or(MeetingType::Internal),
            account: self.account,
            prep: if self.prep.is_empty() {
                None
            } else {
                Some(self.prep)
            },
            is_current: None,
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
pub fn parse_actions(path: &Path) -> Result<Vec<Action>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read actions: {}", e))?;

    let mut actions = Vec::new();
    let mut id_counter = 1;

    for line in content.lines() {
        let line = line.trim();

        // Action format: - [x] P1: Title @Account due:2024-02-05
        // or: - [ ] P2: Title
        if !line.starts_with("- [") {
            continue;
        }

        let is_completed = line.starts_with("- [x]") || line.starts_with("- [X]");
        let rest = line
            .strip_prefix("- [x] ")
            .or_else(|| line.strip_prefix("- [X] "))
            .or_else(|| line.strip_prefix("- [ ] "))
            .unwrap_or("");

        // Parse priority: P1: Title...
        let (priority, rest) = if let Some(r) = rest.strip_prefix("P1:") {
            (Priority::P1, r.trim())
        } else if let Some(r) = rest.strip_prefix("P2:") {
            (Priority::P2, r.trim())
        } else if let Some(r) = rest.strip_prefix("P3:") {
            (Priority::P3, r.trim())
        } else {
            (Priority::P2, rest) // Default to P2
        };

        // Extract account (@Account)
        let mut account: Option<String> = None;
        let mut title = rest.to_string();

        if let Some(at_pos) = rest.find('@') {
            let after_at = &rest[at_pos + 1..];
            let account_end = after_at
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_at.len());
            account = Some(after_at[..account_end].to_string());

            // Remove account from title
            title = format!("{}{}", &rest[..at_pos], &rest[at_pos + 1 + account_end..])
                .trim()
                .to_string();
        }

        // Extract due date (due:YYYY-MM-DD)
        let mut due_date: Option<String> = None;
        if let Some(due_pos) = title.find("due:") {
            let after_due = &title[due_pos + 4..];
            let due_end = after_due
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_due.len());
            let raw_due = &after_due[..due_end];
            due_date = Some(format_due_date(raw_due));

            // Remove due from title
            title = format!("{}{}", &title[..due_pos], &title[due_pos + 4 + due_end..])
                .trim()
                .to_string();
        }

        // Check if overdue (simple check: if due date contains "Yesterday" or past date)
        let is_overdue = due_date
            .as_ref()
            .map(|d| d.contains("Yesterday") || d.contains("Overdue"))
            .unwrap_or(false);

        actions.push(Action {
            id: format!("a{}", id_counter),
            title,
            account,
            due_date,
            priority,
            status: if is_completed {
                ActionStatus::Completed
            } else {
                ActionStatus::Pending
            },
            is_overdue: if is_overdue { Some(true) } else { None },
        });

        id_counter += 1;
    }

    Ok(actions)
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

/// Count files in the inbox directory
pub fn count_inbox(workspace: &Path) -> usize {
    let inbox_path = workspace.join("00-Inbox");
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

/// Calculate day stats from parsed data
pub fn calculate_stats(meetings: &[Meeting], actions: &[Action], inbox_count: usize) -> DayStats {
    let total_meetings = meetings.len();
    let customer_meetings = meetings
        .iter()
        .filter(|m| matches!(m.meeting_type, MeetingType::Customer))
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
