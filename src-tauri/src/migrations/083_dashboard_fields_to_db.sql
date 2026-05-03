-- Promote dashboard.json narrative fields to DB columns.
-- Eliminates filesystem reads from detail page paths.

-- Account narrative fields (previously in dashboard.json)
ALTER TABLE accounts ADD COLUMN company_overview TEXT;
ALTER TABLE accounts ADD COLUMN strategic_programs TEXT;
ALTER TABLE accounts ADD COLUMN notes TEXT;

-- Project narrative fields (previously in dashboard.json)
ALTER TABLE projects ADD COLUMN description TEXT;
ALTER TABLE projects ADD COLUMN milestones TEXT;
ALTER TABLE projects ADD COLUMN notes TEXT;
