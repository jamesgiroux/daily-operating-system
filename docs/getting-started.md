# Getting Started with the Daily Operating System

Welcome! This guide will help you understand what you're setting up, what you need, and how everything works together.

## What Is This?

The Daily Operating System is a productivity framework that uses AI (Claude Code) to help you:
- Prepare for meetings automatically
- Track action items across all your work
- Process documents and meeting transcripts
- Capture your weekly accomplishments
- Stay organized without manual effort

**The key philosophy**: Value shows up without asking. The system does work for you before you even ask.

---

## What You'll Need

### 1. Claude Code Subscription (Required)

Claude Code is the AI-powered command line tool that powers this system.

**Subscription Options:**
- **Claude Pro** ($20/month) - Includes Claude Code access
- **Claude Max** ($100/month) - Higher usage limits for heavy users

**How to get it:**
1. Go to [claude.ai](https://claude.ai)
2. Sign up or log in
3. Subscribe to Pro or Max
4. Install Claude Code: `npm install -g @anthropic-ai/claude-code`

**Note**: Claude Code runs on your computer, keeping your documents private. Your data stays local unless you explicitly share it.

### 2. Google Workspace Access (Free - Optional but Recommended)

The system can connect to your Google Calendar, Gmail, and Sheets to:
- See your meetings and prepare for them
- Scan your email for important items
- Track account/project data in spreadsheets

**Important**:
- This uses Google's **standard APIs**, not Google's AI features
- You won't be charged for Google AI usage
- The connection is free with any Google account
- All the "intelligence" comes from Claude Code, not Google

### 3. A Place to Store Files

You'll need a folder on your computer for your workspace. The setup wizard will help you choose a location like:
- `~/Documents/productivity`
- `~/workspace`

---

## Understanding Markdown Files

This system uses **Markdown files** (`.md`) for everything. Don't worry if you've never heard of Markdown—it's simple!

### What Is Markdown?

Markdown is just plain text with simple formatting. Here's an example:

```markdown
# Meeting Notes - January 15, 2026

## Attendees
- Jane Smith (VP Engineering)
- John Doe (Product Manager)

## Key Discussion Points

### Project Timeline
The team agreed to a **March launch date**. This is critical for Q1 goals.

### Action Items
- [ ] Jane: Send updated specs by Friday
- [ ] John: Schedule follow-up meeting
- [x] Completed: Budget approval received

## Next Steps
1. Review specs when received
2. Prepare demo environment
3. Draft announcement email

---
*Notes taken by: Your Name*
```

### How It Looks

When viewed in a preview tool, that same text looks like a nicely formatted document:

**# becomes a large heading**
**## becomes a smaller heading**
**- items become bullet points**
**1. 2. 3. become numbered lists**
**[ ] becomes an unchecked checkbox**
**[x] becomes a checked checkbox**
****bold** text** becomes **bold**
***italic* text** becomes *italic*

### The Symbols You'll See

| Symbol | What It Does | Example |
|--------|--------------|---------|
| `#` | Heading (big) | `# Meeting Notes` |
| `##` | Subheading | `## Action Items` |
| `###` | Smaller heading | `### Details` |
| `-` | Bullet point | `- First item` |
| `1.` | Numbered list | `1. First step` |
| `**text**` | Bold | `**important**` |
| `*text*` | Italic | `*emphasis*` |
| `[ ]` | Unchecked box | `- [ ] Todo item` |
| `[x]` | Checked box | `- [x] Done item` |
| `---` | Horizontal line | Separates sections |
| `` `code` `` | Code/technical | `` `filename.md` `` |

### Why Markdown?

1. **Simple** - Just text, works anywhere
2. **Future-proof** - Plain text never becomes obsolete
3. **Searchable** - Easy to find things across files
4. **Version-controlled** - Works perfectly with Git
5. **AI-friendly** - Claude Code can read and write it easily

---

## Viewing Your Files

You have several options for viewing markdown files:

### Option 1: Text Editor (See the Code)

Any text editor shows markdown files. You'll see the raw formatting symbols:
- VS Code (free, excellent for this)
- Sublime Text
- Notepad/TextEdit

### Option 2: Obsidian (Free - Recommended for Non-Technical Users)

[Obsidian](https://obsidian.md) is a free app designed for markdown files.

**Benefits:**
- Beautiful preview of your documents
- Switch between "edit" and "preview" modes
- Links between documents work like a wiki
- Free for personal use

**To use with this system:**
1. Download Obsidian from obsidian.md
2. Open your workspace folder as an "Obsidian vault"
3. Browse and edit your files with nice formatting

### Option 3: VS Code with Preview

If you use VS Code:
1. Open any `.md` file
2. Press `Cmd+Shift+V` (Mac) or `Ctrl+Shift+V` (Windows)
3. See the formatted preview

### Option 4: GitHub/GitLab

If you push your workspace to GitHub, it automatically renders markdown beautifully in the browser.

---

## Bringing In Meeting Transcripts

The system works great with meeting transcripts from tools like:

### Gong
Export your call recordings as text transcripts and save to `_inbox/`:
```
_inbox/2026-01-15-acme-call-transcript.md
```

### Fireflies.ai
Download the transcript and save to inbox.

### Otter.ai
Export meeting notes and save to inbox.

### Granola
Export your notes as markdown directly to inbox.

### Fathom
Export transcripts and save to inbox.

### Manual Notes
Just type or paste your notes into a markdown file!

### What Format Do Transcripts Need?

**Raw transcripts are fine!** The system will:
1. Read the raw transcript
2. Generate a clean summary
3. Extract action items
4. Identify decisions made
5. File everything in the right place

You don't need to format or clean up transcripts before importing.

**Example raw transcript:**
```markdown
Call with Acme Corp - January 15, 2026

[00:00] Jane: Hi everyone, thanks for joining.
[00:15] John: Happy to be here. Should we start with the timeline?
[00:22] Jane: Yes, let's do that. We're looking at March for launch.
[00:35] John: That works for our side. We'll need the specs by Friday.
[00:45] Jane: I'll send those over. Can you schedule the follow-up?
[00:52] John: Will do. Let's plan for next Tuesday.
```

The system will transform this into a proper meeting summary with action items extracted.

---

## The Daily Workflow

Once set up, your typical day looks like this:

### Morning (5 minutes)
```
> claude
> /today
```

Claude Code will:
- Show your meetings for the day
- Prepare context for customer/client calls
- Surface action items due today
- Highlight important emails
- Suggest what to focus on

### End of Day (5 minutes)
```
> /wrap
```

Claude Code will:
- Check if meeting transcripts were processed
- Ask about action items due today
- Capture any wins or accomplishments
- Archive today's files
- Prepare for tomorrow

### Monday Morning (10 minutes)
```
> /week
```

Claude Code will:
- Show all meetings this week
- Surface overdue items
- Help plan your time blocks
- Pre-fill your weekly impact template

---

## What Happens During Setup

When you run the setup wizard (`python3 advanced-start.py` or `easy-start.command`), here's what happens:

### Step 1: Prerequisites Check
- Verifies Python is installed
- Checks for Claude Code CLI
- Checks for Git

### Step 2: Choose Workspace Location
- You pick where to create your productivity folder
- Default: `~/Documents/productivity`

### Step 3: Create Folder Structure
Creates the PARA organization system:
```
productivity/
├── Projects/     # Active work with deadlines
├── Areas/        # Ongoing responsibilities
├── Resources/    # Reference materials
├── Archive/      # Completed items
├── _inbox/       # Where new files go
├── _today/       # Daily working files
└── ...
```

**Different roles may want different structures** - see "Customizing for Your Role" below.

### Step 4: Git Setup (Optional)
- Initializes version control
- Creates `.gitignore` for sensitive files
- Enables backup to GitHub

### Step 5: Google API (Optional)
- Guides you through Google Cloud setup
- Connects Calendar, Gmail, Sheets
- Saves credentials securely

### Step 6: CLAUDE.md Configuration
- Creates your personalized Claude Code config
- Captures your preferences and working style
- Tells Claude Code how to help you best

### Step 7: Install Commands & Skills
- Installs `/today`, `/wrap`, `/week`, etc.
- Sets up specialized workflows
- Configures AI agents

### Step 8: Verification
- Confirms everything is set up correctly
- Shows you what's ready to use

---

## After Setup: First Steps

### 1. Start the Dashboard (Optional)
If you installed the `dailyos` CLI:
```bash
dailyos start                    # Opens web dashboard in browser
```

Or manually:
```bash
cd ~/Documents/productivity/_ui
npm start
# Then open http://localhost:5050
```

### 2. Try `/today`
```
cd ~/Documents/productivity  # or wherever you set up
claude
/today
```

### 3. Add a Test Transcript
Save a meeting transcript to `_inbox/`:
```
_inbox/2026-01-15-test-meeting-transcript.md
```

### 4. Process It
```
/inbox
```

### 5. See the Results
Check the organized output in your folders!

---

## The DailyOS CLI

If you installed the `dailyos` command during setup, you have access to these utilities:

| Command | What It Does |
|---------|--------------|
| `dailyos start` | Start the web dashboard |
| `dailyos stop` | Stop the web dashboard |
| `dailyos ui` | Check if dashboard is running |
| `dailyos version` | Show version info |
| `dailyos status` | Check for updates |
| `dailyos update` | Update to latest version |
| `dailyos doctor` | Check workspace health |
| `dailyos repair` | Fix broken installation |

See [cli-reference.md](cli-reference.md) for full documentation.

---

## Getting Help

### In Claude Code
- Type `/help` for available commands
- Ask Claude Code questions naturally

### Documentation
- Open `ui/index.html` in a browser for visual docs
- Check the `docs/` folder for detailed guides

### Common Issues

**"Claude Code not found"**
→ Install it: `npm install -g @anthropic-ai/claude-code`

**"Google API not working"**
→ Re-run: `python3 advanced-start.py --google`

**"Where are my files?"**
→ Check `_today/` for today's files
→ Check `_inbox/` for unprocessed items

---

## Customizing for Your Role

The default setup assumes you **own accounts or projects** over time. Here are structures for different roles:

### Customer Success Managers (CSMs)

Own a portfolio of accounts long-term:
```
Accounts/
├── ClientA/
├── ClientB/
└── ClientC/
```

### Account Executives (AEs)

Track deals through pipeline stages:
```
Accounts/
├── Discovery/        # Early conversations
├── Qualified/        # Active opportunities
├── Negotiating/      # Close to signing
├── Closed-Won/       # Customers (hand off to CS)
└── Closed-Lost/      # Lost deals (for learnings)
```

**To change after setup**, ask Claude Code:
```
"Reorganize my Accounts folder into sales pipeline stages:
Discovery, Qualified, Negotiating, Closed-Won, Closed-Lost"
```

### Sales Development Reps (SDRs/BDRs)

Work with many prospects briefly:
```
Accounts/
├── Active/           # Currently pursuing
├── Qualified/        # Handed off to AE
├── Disqualified/     # Not a fit
└── Nurture/          # Future pipeline
```

**To change after setup**, ask Claude Code:
```
"Reorganize my Accounts folder for SDR workflow with Active,
Qualified, Disqualified, and Nurture subdirectories"
```

### Project Managers

Organize by project lifecycle:
```
Projects/
├── Active/           # In progress
├── Planning/         # Upcoming
├── On-Hold/          # Paused
└── Completed/        # Archived
```

**To change after setup**, ask Claude Code:
```
"Set up project lifecycle folders: Active, Planning, On-Hold, Completed"
```

### Consultants

Handle engagements with defined start/end:
```
Engagements/
├── Current/          # Active projects
├── Pipeline/         # Upcoming work
└── Completed/        # Past engagements
```

**To change after setup**, ask Claude Code:
```
"Create an engagement structure with Current, Pipeline, and Completed folders"
```

### Marketers

Organize by campaign status:
```
Campaigns/
├── Active/           # Running now
├── Planned/          # In development
├── Completed/        # Finished (with results)
└── Templates/        # Reusable frameworks
```

### Prompt Examples for Customization

After setup, ask Claude Code to customize further:

**Add last-contact tracking:**
```
"Add a Last-Contact-Date field to each account's frontmatter
and create a view that shows accounts by recency of engagement"
```

**Create a handoff tracker:**
```
"Create a handoff-tracker.md file with columns for Account,
From, To, Date, and Status"
```

**Set up active/inactive workflow:**
```
"When I mark an account as inactive, move it to Accounts/Inactive/
and archive its action items. When I reactivate, move it back."
```

---

## Privacy & Security

- **Your files stay local** - Nothing is uploaded unless you choose to
- **Transcripts are processed locally** - Claude Code reads them on your machine
- **Google API is read-mostly** - Only creates drafts, never sends emails automatically
- **Git is optional** - Only push to GitHub if you want backup

---

## Next Steps

1. Run the setup wizard: `python3 advanced-start.py` (or double-click `easy-start.command`)
2. Follow the prompts
3. Try `/today` tomorrow morning
4. Process your first transcript
5. Enjoy your new productivity system!

Welcome aboard!
