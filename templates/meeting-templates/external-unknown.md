# Template: External Unknown

## When Used

Applied when a meeting has external attendees (different email domain from user) but those attendees do not match any known entity -- not a tracked account, not a recognized partner. This is the fallback for external meetings that the classification system cannot categorize.

Per DEC22: The system should proactively research unknown meetings rather than leaving them blank or asking the user to fill in gaps. "The system operates. You leverage."

## Required Context (from directive refs)

1. **Meeting event data** -- From directive JSON: title, time, duration, attendee list (names and emails), calendar description
2. **Attendee domains** -- Extract the company domain from attendee email addresses (e.g., `j.smith@newco.com` yields `newco.com`)
3. **Local archive search** -- Search the entire workspace archive for any prior interactions with:
   - The attendee email addresses
   - The attendee domain
   - The company name (inferred from domain)
   - Names of the attendees
4. **Calendar description** -- Often contains context about why the meeting was scheduled

Claude should perform the research steps below. This template requires more inference than others because structured data does not exist for these contacts.

## Research Steps

Before generating output, Claude should:

1. **Identify the company** from the email domain. Strip common patterns (`mail.`, `corporate.`) to get the root domain.
2. **Search the workspace** for any file that mentions this domain, company name, or attendee names. Check `_archive/`, `_inbox/`, `_reference/`, `Accounts/`, `Projects/`.
3. **Check for indirect connections** -- Is this domain a subsidiary of a known account? A vendor? A recruiter? Context from the calendar description often reveals the purpose.
4. **Infer the meeting purpose** from the title, description, and any workspace matches.

## Output Sections

### 1. Research Brief

What the system could determine about the external party:

**Company:** {Name} ({domain})
**Likely industry:** {if inferable from domain/context}
**Meeting purpose:** {inferred from title + description + any workspace matches}

If the calendar description provides context (e.g., "Intro call to discuss potential partnership"), lead with that.

If nothing is known beyond the domain: "Limited information available. {domain} does not match any tracked accounts, partners, or prior interactions."

### 2. Attendee Profiles

For each external attendee:

- **{Name}** ({email}) -- {any context found: title from email signature in archive, prior interaction notes, role inferred from calendar description}

If no context exists for an attendee: "No prior interaction history found."

For internal attendees on the invite (besides the user), note who they are -- a colleague's presence may indicate the meeting's purpose (e.g., a sales rep on the call suggests a prospect meeting).

### 3. Known History

Any prior interactions found in the workspace:

- **Email thread found:** {subject, date} in `_archive/2026-01-15/`
- **Inbox file:** `_inbox/newco-proposal.md` processed on Jan 20
- **Mention in notes:** Referenced in `Projects/expansion/research.md`

If no history found: "No prior interactions found in workspace archive."

### 4. Suggested Approach

Based on available context, suggest how to approach the meeting:

- **If prospect/sales context detected:** "This appears to be a prospect conversation. Consider: what do they need, what's the decision timeline, who else is involved."
- **If vendor/pitch context detected:** "This appears to be an inbound pitch. Decide in advance whether this is worth evaluating."
- **If referral/intro detected:** "This appears to be a warm introduction. Note who made the intro and what they told both parties."
- **If truly unknown:** "Purpose unclear. Suggested opening: confirm the meeting objective and attendee roles in the first 2 minutes."

Keep suggestions practical, not generic. One specific suggestion is better than five vague ones.

## Formatting Guidelines

- First line: `# {Meeting Title}`
- Second line: `**Time:** {start} - {end}`
- Third line: `**External:** {company name or domain} | {count} external attendees`
- Target length: 200-350 words
- Tone: Investigative but not paranoid -- the goal is context, not surveillance
- Clearly distinguish between facts (found in workspace) and inferences (derived from domain/title)
- Use "(inferred)" labels when context is derived rather than sourced

## Profile Variations

- **CSM:** Add a check: Could this company be a prospect, a customer's subsidiary, or a partner's referral? Cross-reference the domain against:
  - Account contact domains (partial match -- same parent company?)
  - Partner referral pipeline (was this company mentioned?)
  - Recently churned accounts (is this a win-back?)

  If a potential connection is found: "Possible connection: {domain} may be related to {Account Name} -- similar industry, overlapping contacts."

- **General:** Focus on who they are and what they likely want. Skip account/prospect analysis. If internal colleagues are on the invite, note their typical role (e.g., "Your colleague Sarah from Sales is also attending -- this may be a business development meeting"). Check `Projects/` for any project that mentions the company or domain.
