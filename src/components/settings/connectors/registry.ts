import type { ComponentType } from "react";
import GoogleConnector from "./GoogleConnector";
import ClaudeDesktopConnector from "./ClaudeDesktopConnector";
import QuillConnector from "./QuillConnector";
import GranolaConnector from "./GranolaConnector";
import GravatarConnector from "./GravatarConnector";
import ClayConnector from "./ClayConnector";
import LinearConnector from "./LinearConnector";

export interface ConnectorEntry {
  id: string;
  name: string;
  component: ComponentType;
  statusCommand: string;
}

export const connectors: ConnectorEntry[] = [
  { id: "google", name: "Google", component: GoogleConnector, statusCommand: "get_google_auth_status" },
  { id: "claude-desktop", name: "Claude Desktop", component: ClaudeDesktopConnector, statusCommand: "get_claude_desktop_status" },
  { id: "quill", name: "Quill Transcripts", component: QuillConnector, statusCommand: "get_quill_status" },
  { id: "granola", name: "Granola Transcripts", component: GranolaConnector, statusCommand: "get_granola_status" },
  { id: "gravatar", name: "Gravatar Avatars", component: GravatarConnector, statusCommand: "get_gravatar_status" },
  { id: "clay", name: "Clay Enrichment", component: ClayConnector, statusCommand: "get_clay_status" },
  { id: "linear", name: "Linear Issues", component: LinearConnector, statusCommand: "get_linear_status" },
];
