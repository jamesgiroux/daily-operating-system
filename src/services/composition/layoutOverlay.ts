import { invoke } from "@tauri-apps/api/core";

export type CompositionEntityType = "account" | "project" | "person";
export type CompositionSurfaceKey = "entity_page";
export type CompositionBlockVariant = "default" | "compact" | "spotlight";

export interface CompositionLayoutOverlay {
  schemaVersion: 1;
  sectionOrder: string[];
  blockOrder: Record<string, string[]>;
  hiddenSectionIds: string[];
  hiddenBlockIds: string[];
  blockVariants: Record<string, CompositionBlockVariant>;
  sectionLabelOverrides: Record<string, string>;
  updatedAt?: string | null;
}

export interface LayoutOverlayKey {
  entityType: CompositionEntityType;
  surfaceKey: CompositionSurfaceKey;
}

export interface LayoutOverlayResponse extends LayoutOverlayKey {
  overlaySchemaVersion: number;
  layoutRevision: number;
  overlay: CompositionLayoutOverlay | null;
  updatedAt?: string | null;
}

export interface SaveLayoutOverlayRequest extends LayoutOverlayKey {
  overlay: CompositionLayoutOverlay;
}

export const DEFAULT_COMPOSITION_LAYOUT_OVERLAY: CompositionLayoutOverlay = {
  schemaVersion: 1,
  sectionOrder: [],
  blockOrder: {},
  hiddenSectionIds: [],
  hiddenBlockIds: [],
  blockVariants: {},
  sectionLabelOverrides: {},
};

export function normalizeCompositionLayoutOverlay(
  overlay: CompositionLayoutOverlay | null | undefined,
): CompositionLayoutOverlay {
  if (!overlay) return { ...DEFAULT_COMPOSITION_LAYOUT_OVERLAY };
  return {
    schemaVersion: 1,
    sectionOrder: Array.isArray(overlay.sectionOrder) ? overlay.sectionOrder : [],
    blockOrder: overlay.blockOrder && typeof overlay.blockOrder === "object" ? overlay.blockOrder : {},
    hiddenSectionIds: Array.isArray(overlay.hiddenSectionIds) ? overlay.hiddenSectionIds : [],
    hiddenBlockIds: Array.isArray(overlay.hiddenBlockIds) ? overlay.hiddenBlockIds : [],
    blockVariants: overlay.blockVariants && typeof overlay.blockVariants === "object" ? overlay.blockVariants : {},
    sectionLabelOverrides:
      overlay.sectionLabelOverrides && typeof overlay.sectionLabelOverrides === "object"
        ? overlay.sectionLabelOverrides
        : {},
    updatedAt: overlay.updatedAt ?? null,
  };
}

export async function getCompositionLayoutOverlay(
  key: LayoutOverlayKey,
): Promise<LayoutOverlayResponse> {
  return invoke<LayoutOverlayResponse>("get_composition_layout_overlay", { key });
}

export async function saveCompositionLayoutOverlay(
  request: SaveLayoutOverlayRequest,
): Promise<LayoutOverlayResponse> {
  return invoke<LayoutOverlayResponse>("save_composition_layout_overlay", { request });
}

export async function resetCompositionLayoutOverlay(
  key: LayoutOverlayKey,
): Promise<LayoutOverlayResponse> {
  return invoke<LayoutOverlayResponse>("reset_composition_layout_overlay", { key });
}
