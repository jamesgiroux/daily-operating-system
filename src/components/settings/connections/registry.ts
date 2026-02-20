import type { ComponentType } from "react";
import GoogleConnection from "./GoogleConnection";
import ClaudeDesktopConnection from "./ClaudeDesktopConnection";
import QuillConnection from "./QuillConnection";
import GranolaConnection from "./GranolaConnection";
import GravatarConnection from "./GravatarConnection";
import ClayConnection from "./ClayConnection";
import LinearConnection from "./LinearConnection";

export interface ConnectionEntry {
  id: string;
  name: string;
  component: ComponentType;
  statusCommand: string;
}

export const connections: ConnectionEntry[] = [
  { id: "google", name: "Google", component: GoogleConnection, statusCommand: "get_google_auth_status" },
  { id: "claude-desktop", name: "Claude Desktop", component: ClaudeDesktopConnection, statusCommand: "get_claude_desktop_status" },
  { id: "quill", name: "Quill Transcripts", component: QuillConnection, statusCommand: "get_quill_status" },
  { id: "granola", name: "Granola Transcripts", component: GranolaConnection, statusCommand: "get_granola_status" },
  { id: "gravatar", name: "Gravatar Avatars", component: GravatarConnection, statusCommand: "get_gravatar_status" },
  { id: "clay", name: "Clay Enrichment", component: ClayConnection, statusCommand: "get_clay_status" },
  { id: "linear", name: "Linear Issues", component: LinearConnection, statusCommand: "get_linear_status" },
];
