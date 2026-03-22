import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

import {
  PHASE3_SURFACE_CONTRACTS,
  type ParityDataset,
  type UiErrorPayload,
} from "./phase3ContractRegistry";
import { PHASE3_SURFACE_OWNERSHIP } from "./phase3OwnershipMap";

interface ParityFixture {
  surfaceId: string;
  commands: Record<string, unknown>;
  errors?: Record<string, UiErrorPayload>;
}

const DATASET_DIR: Record<ParityDataset, string> = {
  mock: path.resolve(process.cwd(), ".docs/fixtures/parity/mock"),
  production: path.resolve(process.cwd(), ".docs/fixtures/parity/production"),
};
const CONTRACT_REGISTRY_ARTIFACT = path.resolve(
  process.cwd(),
  ".docs/contracts/phase3-ui-contract-registry.json"
);
const OWNERSHIP_MAP_ARTIFACT = path.resolve(
  process.cwd(),
  ".docs/contracts/phase3-ui-ownership-map.json"
);
const ROUTER_FILE = path.resolve(process.cwd(), "src/router.tsx");

function loadFixture(dataset: ParityDataset, surfaceId: string): ParityFixture {
  const file = path.join(DATASET_DIR[dataset], `${surfaceId}.json`);
  const raw = fs.readFileSync(file, "utf8");
  return JSON.parse(raw) as ParityFixture;
}

function loadJsonFile<T>(file: string): T {
  return JSON.parse(fs.readFileSync(file, "utf8")) as T;
}

function getPathValue(target: unknown, dotPath: string): unknown {
  const parts = dotPath.split(".");
  let cursor: unknown = target;
  for (const part of parts) {
    if (cursor == null) return undefined;
    if (Array.isArray(cursor)) {
      const index = Number(part);
      if (!Number.isInteger(index)) return undefined;
      cursor = cursor[index];
      continue;
    }
    if (typeof cursor === "object") {
      cursor = (cursor as Record<string, unknown>)[part];
      continue;
    }
    return undefined;
  }
  return cursor;
}

function flattenShapePaths(value: unknown, parent = ""): Set<string> {
  const out = new Set<string>();
  if (Array.isArray(value)) {
    out.add(parent);
    const sample = value[0];
    if (sample !== undefined) {
      const prefix = parent ? `${parent}.*` : "*";
      for (const p of flattenShapePaths(sample, prefix)) out.add(p);
    }
    return out;
  }
  if (value && typeof value === "object") {
    for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
      const next = parent ? `${parent}.${key}` : key;
      out.add(next);
      for (const p of flattenShapePaths(nested, next)) out.add(p);
    }
    return out;
  }
  if (parent) out.add(parent);
  return out;
}

function normalizePath(pathValue: string): string {
  return pathValue.replace(/\.\d+(\.|$)/g, ".*$1");
}

describe("Phase 3 parity gate", () => {
  it("keeps the committed contract registry artifact in sync with the TypeScript registry", () => {
    const artifact = loadJsonFile<{ surfaces: typeof PHASE3_SURFACE_CONTRACTS }>(CONTRACT_REGISTRY_ARTIFACT);
    expect(artifact.surfaces).toEqual(PHASE3_SURFACE_CONTRACTS);
  });

  it("has fixture files for every major surface in mock and production datasets", () => {
    for (const surface of PHASE3_SURFACE_CONTRACTS) {
      const mockFile = path.join(DATASET_DIR.mock, `${surface.id}.json`);
      const prodFile = path.join(DATASET_DIR.production, `${surface.id}.json`);
      expect(fs.existsSync(mockFile), `missing mock fixture for ${surface.id}`).toBe(true);
      expect(fs.existsSync(prodFile), `missing production fixture for ${surface.id}`).toBe(true);
    }
  });

  it("satisfies required contract paths for all declared commands", () => {
    for (const surface of PHASE3_SURFACE_CONTRACTS) {
      for (const consumer of surface.consumers) {
        const file = path.resolve(process.cwd(), "src", consumer);
        expect(fs.existsSync(file), `missing consumer file ${consumer} for ${surface.id}`).toBe(true);
      }
      for (const dataset of ["mock", "production"] as const) {
        const fixture = loadFixture(dataset, surface.id);
        expect(fixture.surfaceId).toBe(surface.id);
        for (const command of surface.commands) {
          expect(
            fixture.commands[command.command],
            `${surface.id}/${dataset}: missing command payload ${command.command}`
          ).toBeDefined();
          for (const requiredPath of command.requiredPaths) {
            const value = getPathValue(fixture.commands[command.command], requiredPath);
            expect(
              value,
              `${surface.id}/${dataset}/${command.command}: missing required path ${requiredPath}`
            ).not.toBeUndefined();
          }
        }
      }
    }
  });

  it("keeps the ownership map artifact in sync and covers every parity surface route", () => {
    const artifact = loadJsonFile<{ surfaces: typeof PHASE3_SURFACE_OWNERSHIP }>(OWNERSHIP_MAP_ARTIFACT);
    const routerSource = fs.readFileSync(ROUTER_FILE, "utf8");

    expect(artifact.surfaces).toEqual(PHASE3_SURFACE_OWNERSHIP);
    expect(PHASE3_SURFACE_OWNERSHIP.map((surface) => surface.id)).toEqual(
      PHASE3_SURFACE_CONTRACTS.map((surface) => surface.id)
    );

    for (const surface of PHASE3_SURFACE_OWNERSHIP) {
      expect(surface.routes.length, `${surface.id}: missing routed ownership`).toBeGreaterThan(0);
      expect(surface.owners.length, `${surface.id}: missing owner files`).toBeGreaterThan(0);

      for (const owner of surface.owners) {
        const file = path.resolve(process.cwd(), "src", owner);
        expect(fs.existsSync(file), `missing owner file ${owner} for ${surface.id}`).toBe(true);
      }

      for (const route of surface.routes) {
        expect(
          routerSource.includes(`path: "${route}"`),
          `${surface.id}: route ${route} missing from router`
        ).toBe(true);
      }
    }
  });

  it("prevents mock-only shape dependencies (mock payload keys must exist in production payloads)", () => {
    for (const surface of PHASE3_SURFACE_CONTRACTS) {
      const mock = loadFixture("mock", surface.id);
      const production = loadFixture("production", surface.id);

      for (const command of surface.commands) {
        const mockPaths = [...flattenShapePaths(mock.commands[command.command])].map(normalizePath);
        const prodPathSet = new Set(
          [...flattenShapePaths(production.commands[command.command])].map(normalizePath)
        );

        for (const mockPath of mockPaths) {
          expect(
            prodPathSet.has(mockPath),
            `${surface.id}/${command.command}: mock-only path '${mockPath}' not present in production shape`
          ).toBe(true);
        }
      }
    }
  });

  it("enforces consistent UI error payload shape", () => {
    for (const surface of PHASE3_SURFACE_CONTRACTS) {
      for (const dataset of ["mock", "production"] as const) {
        const fixture = loadFixture(dataset, surface.id);
        if (!fixture.errors) continue;
        for (const [command, error] of Object.entries(fixture.errors)) {
          expect(typeof command).toBe("string");
          expect(typeof error.code).toBe("string");
          expect(typeof error.message).toBe("string");
          expect(typeof error.retryable).toBe("boolean");
        }
      }
    }
  });

  it("requires actions/proposed-actions visibility with production-shaped data", () => {
    for (const dataset of ["mock", "production"] as const) {
      const fixture = loadFixture(dataset, "actions");
      const dbActions = fixture.commands.get_actions_from_db as Array<{
        id?: string;
        title?: string;
        priority?: string;
        status?: string;
      }>;
      const suggestedActions = fixture.commands.get_suggested_actions as Array<{
        id?: string;
        title?: string;
        priority?: string;
        status?: string;
      }>;

      expect(Array.isArray(dbActions)).toBe(true);
      expect(Array.isArray(suggestedActions)).toBe(true);
      expect(dbActions.length, `${dataset}: get_actions_from_db empty`).toBeGreaterThan(0);
      expect(suggestedActions.length, `${dataset}: get_suggested_actions empty`).toBeGreaterThan(0);

      for (const row of [...dbActions, ...suggestedActions]) {
        expect(typeof row.id).toBe("string");
        expect(typeof row.title).toBe("string");
        expect(typeof row.priority).toBe("string");
        expect(typeof row.status).toBe("string");
      }
    }
  });
});
