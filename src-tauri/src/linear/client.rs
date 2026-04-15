//! GraphQL client for the Linear API.
//!
//! Uses reqwest with Bearer token auth. All queries target
//! `https://api.linear.app/graphql`.

use serde::{Deserialize, Serialize};

const LINEAR_API_URL: &str = "https://api.linear.app/graphql";

/// Viewer info returned by test_connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearViewer {
    pub name: String,
    pub email: String,
}

/// A Linear issue assigned to the current user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub state_name: Option<String>,
    pub state_type: Option<String>,
    pub priority: Option<i32>,
    pub priority_label: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub due_date: Option<String>,
    pub url: String,
}

/// A Linear project the user is a member of.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearProject {
    pub id: String,
    pub name: String,
    pub state: Option<String>,
    pub url: String,
}

/// A Linear team.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearTeam {
    pub id: String,
    pub name: String,
}

/// Result of creating an issue via the Linear API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearCreatedIssue {
    pub id: String,
    pub identifier: String,
    pub title: String,
    pub url: String,
}

pub struct LinearClient {
    client: reqwest::Client,
    api_key: String,
}

impl LinearClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
        }
    }

    async fn graphql<T: serde::de::DeserializeOwned>(&self, query: &str) -> Result<T, String> {
        let body = serde_json::json!({ "query": query });
        self.execute_graphql(body).await
    }

    async fn graphql_with_vars<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T, String> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        self.execute_graphql(body).await
    }

    async fn execute_graphql<T: serde::de::DeserializeOwned>(
        &self,
        body: serde_json::Value,
    ) -> Result<T, String> {
        let resp = self
            .client
            .post(LINEAR_API_URL)
            .header("Authorization", self.api_key.clone())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Linear API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Linear API error {}: {}", status, text));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Linear response: {}", e))?;

        if let Some(errors) = json.get("errors") {
            return Err(format!("Linear GraphQL errors: {}", errors));
        }

        let data = json
            .get("data")
            .ok_or("Missing 'data' in Linear response")?;

        serde_json::from_value(data.clone())
            .map_err(|e| format!("Failed to deserialize Linear data: {}", e))
    }

    /// Test connection by fetching the authenticated viewer.
    pub async fn test_connection(&self) -> Result<LinearViewer, String> {
        #[derive(Deserialize)]
        struct ViewerResponse {
            viewer: LinearViewer,
        }

        let resp: ViewerResponse = self.graphql("{ viewer { name email } }").await?;

        Ok(resp.viewer)
    }

    /// Fetch issues assigned to the current user that are not completed/cancelled.
    pub async fn fetch_my_issues(&self) -> Result<Vec<LinearIssue>, String> {
        #[derive(Deserialize)]
        struct IssuesResponse {
            viewer: ViewerIssues,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ViewerIssues {
            assigned_issues: IssueConnection,
        }
        #[derive(Deserialize)]
        struct IssueConnection {
            nodes: Vec<IssueNode>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            state: Option<StateNode>,
            priority: Option<i32>,
            priority_label: Option<String>,
            project: Option<ProjectRef>,
            due_date: Option<String>,
            url: String,
        }
        #[derive(Deserialize)]
        struct StateNode {
            name: Option<String>,
            #[serde(rename = "type")]
            state_type: Option<String>,
        }
        #[derive(Deserialize)]
        struct ProjectRef {
            id: String,
            name: String,
        }

        let query = r#"{
            viewer {
                assignedIssues(
                    filter: { state: { type: { nin: ["completed", "cancelled"] } } }
                    first: 100
                    orderBy: updatedAt
                ) {
                    nodes {
                        id identifier title
                        state { name type }
                        priority priorityLabel
                        project { id name }
                        dueDate url
                    }
                }
            }
        }"#;

        let resp: IssuesResponse = self.graphql(query).await?;

        Ok(resp
            .viewer
            .assigned_issues
            .nodes
            .into_iter()
            .map(|n| LinearIssue {
                id: n.id,
                identifier: n.identifier,
                title: n.title,
                state_name: n.state.as_ref().and_then(|s| s.name.clone()),
                state_type: n.state.as_ref().and_then(|s| s.state_type.clone()),
                priority: n.priority,
                priority_label: n.priority_label,
                project_id: n.project.as_ref().map(|p| p.id.clone()),
                project_name: n.project.as_ref().map(|p| p.name.clone()),
                due_date: n.due_date,
                url: n.url,
            })
            .collect())
    }

    /// Fetch projects from teams the user is a member of.
    pub async fn fetch_my_projects(&self) -> Result<Vec<LinearProject>, String> {
        #[derive(Deserialize)]
        struct TeamsResponse {
            teams: TeamConnection,
        }
        #[derive(Deserialize)]
        struct TeamConnection {
            nodes: Vec<TeamNode>,
        }
        #[derive(Deserialize)]
        struct TeamNode {
            projects: ProjectConnection,
        }
        #[derive(Deserialize)]
        struct ProjectConnection {
            nodes: Vec<ProjectNode>,
        }
        #[derive(Deserialize)]
        struct ProjectNode {
            id: String,
            name: String,
            state: Option<String>,
            url: String,
        }

        let query = r#"{
            teams {
                nodes {
                    projects(first: 50) {
                        nodes {
                            id name state url
                        }
                    }
                }
            }
        }"#;

        let resp: TeamsResponse = self.graphql(query).await?;

        let mut seen = std::collections::HashSet::new();
        let mut projects = Vec::new();
        for team in resp.teams.nodes {
            for p in team.projects.nodes {
                if seen.insert(p.id.clone()) {
                    projects.push(LinearProject {
                        id: p.id,
                        name: p.name,
                        state: p.state,
                        url: p.url,
                    });
                }
            }
        }

        Ok(projects)
    }

    /// Fetch all teams the authenticated user belongs to.
    pub async fn fetch_teams(&self) -> Result<Vec<LinearTeam>, String> {
        #[derive(Deserialize)]
        struct TeamsResponse {
            teams: TeamConnection,
        }
        #[derive(Deserialize)]
        struct TeamConnection {
            nodes: Vec<LinearTeam>,
        }

        let query = r#"{
            teams {
                nodes {
                    id name
                }
            }
        }"#;

        let resp: TeamsResponse = self.graphql(query).await?;
        Ok(resp.teams.nodes)
    }

    /// Create a new issue in Linear.
    pub async fn create_issue(
        &self,
        title: &str,
        team_id: &str,
        description: Option<&str>,
        project_id: Option<&str>,
        priority: Option<i32>,
        due_date: Option<&str>,
    ) -> Result<LinearCreatedIssue, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct IssueCreateResponse {
            issue_create: IssueCreatePayload,
        }
        #[derive(Deserialize)]
        struct IssueCreatePayload {
            success: bool,
            issue: Option<IssueNode>,
        }
        #[derive(Deserialize)]
        struct IssueNode {
            id: String,
            identifier: String,
            title: String,
            url: String,
        }

        let mutation = r#"
            mutation IssueCreate($input: IssueCreateInput!) {
                issueCreate(input: $input) {
                    success
                    issue {
                        id
                        identifier
                        title
                        url
                    }
                }
            }
        "#;

        let mut input = serde_json::json!({
            "title": title,
            "teamId": team_id,
        });

        if let Some(desc) = description {
            input["description"] = serde_json::Value::String(desc.to_string());
        }
        if let Some(pid) = project_id {
            input["projectId"] = serde_json::Value::String(pid.to_string());
        }
        if let Some(p) = priority {
            input["priority"] = serde_json::Value::Number(serde_json::Number::from(p));
        }
        if let Some(dd) = due_date {
            input["dueDate"] = serde_json::Value::String(dd.to_string());
        }

        let variables = serde_json::json!({ "input": input });
        let resp: IssueCreateResponse = self.graphql_with_vars(mutation, variables).await?;

        if !resp.issue_create.success {
            return Err("Linear issueCreate returned success=false".to_string());
        }

        let issue = resp
            .issue_create
            .issue
            .ok_or("Linear issueCreate succeeded but returned no issue")?;

        Ok(LinearCreatedIssue {
            id: issue.id,
            identifier: issue.identifier,
            title: issue.title,
            url: issue.url,
        })
    }
}
