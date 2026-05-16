import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { TrustBand } from "@/components/intelligence/TrustBand";
import type { TrustBand as TrustBandValue } from "@/components/ui/TrustBandBadge";
import {
  EntityListEmpty,
  EntityListError,
  EntityListSkeleton,
} from "@/components/entity/EntityListShell";
import { EntityRow } from "@/components/entity/EntityRow";
import styles from "./LinearIssuesChapter.module.css";

export type LinearIssueEntityRef = {
  kind: "account" | "project";
  id: string;
};

export type LinearIssueActorScope = "user" | "agent" | "system";

interface LinearIssuesChapterProps {
  entityRef: LinearIssueEntityRef;
  actorScope: LinearIssueActorScope;
}

type LinearIssueStateGroup = "open" | "in_progress" | "blocked" | "done";

interface LinearEntityIssue {
  id: string;
  identifier: string | null;
  title: string;
  stateName: string | null;
  stateType: string | null;
  stateGroup: LinearIssueStateGroup;
  priority: number | null;
  priorityLabel: string | null;
  projectId: string | null;
  projectName: string | null;
  assigneeName: string | null;
  dueDate: string | null;
  url: string | null;
  sourceRef: string;
  sourceAsof: string | null;
  trustBand: string;
  sourceLifecycleState: string;
  redacted: boolean;
}

const GROUPS: Array<{ key: LinearIssueStateGroup; label: string }> = [
  { key: "open", label: "Open" },
  { key: "in_progress", label: "In Progress" },
  { key: "blocked", label: "Blocked" },
  { key: "done", label: "Done" },
];

const TRUST_BANDS: TrustBandValue[] = [
  "likely_current",
  "use_with_caution",
  "needs_verification",
];

export function LinearIssuesChapter({
  entityRef,
  actorScope,
}: LinearIssuesChapterProps) {
  const [issues, setIssues] = useState<LinearEntityIssue[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    invoke<LinearEntityIssue[]>("get_linear_issues_for_entity", {
      entityType: entityRef.kind,
      entityId: entityRef.id,
      actorScope,
    })
      .then((nextIssues) => {
        if (!cancelled) {
          setIssues(nextIssues);
        }
      })
      .catch((reason) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [actorScope, entityRef.id, entityRef.kind, retryCount]);

  const groupedIssues = useMemo(() => {
    return GROUPS.map((group) => ({
      ...group,
      issues: issues.filter((issue) => issue.stateGroup === group.key),
    })).filter((group) => group.issues.length > 0);
  }, [issues]);

  return (
    <section
      id="linear-issues"
      className={`editorial-reveal ${styles.chapter}`}
    >
      <ChapterHeading
        title="Linear Issues"
        epigraph="Linear-sourced work grouped by current state"
      />

      {loading ? (
        <EntityListSkeleton />
      ) : error ? (
        <EntityListError
          error={error}
          onRetry={() => setRetryCount((count) => count + 1)}
        />
      ) : groupedIssues.length === 0 ? (
        <EntityListEmpty
          title="No Linear issues"
          message="No Linear-sourced work is linked to this page yet."
        />
      ) : (
        <div className={styles.groups}>
          {groupedIssues.map((group) => (
            <section key={group.key} className={styles.group}>
              <div className={styles.groupHeader}>
                <h3 className={styles.groupTitle}>{group.label}</h3>
                <span className={styles.groupSource}>Linear-sourced</span>
              </div>
              <div className={styles.rows}>
                {group.issues.map((issue, index) => (
                  <LinearIssueRow
                    key={`${issue.sourceRef}:${issue.id}`}
                    issue={issue}
                    stateGroup={group.key}
                    showBorder={index < group.issues.length - 1}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}

      <FinisMarker />
    </section>
  );
}

function LinearIssueRow({
  issue,
  stateGroup,
  showBorder,
}: {
  issue: LinearEntityIssue;
  stateGroup: LinearIssueStateGroup;
  showBorder: boolean;
}) {
  const nameSuffix = issue.identifier ? (
    <span className={styles.identifier}>{issue.identifier}</span>
  ) : null;
  const trustBand = toTrustBand(issue.trustBand);

  return (
    <EntityRow
      href={issue.redacted ? null : issue.url}
      name={issue.title}
      showBorder={showBorder}
      avatar={
        <span
          className={styles.stateDot}
          data-state={stateGroup}
          aria-hidden="true"
        />
      }
      nameSuffix={nameSuffix}
      subtitle={<LinearIssueSubtitle issue={issue} />}
    >
      {trustBand ? (
        <TrustBand
          band={trustBand}
          source="linear"
          asOf={issue.sourceAsof}
          density="compact"
          align="row"
        />
      ) : null}
    </EntityRow>
  );
}

function LinearIssueSubtitle({ issue }: { issue: LinearEntityIssue }) {
  if (issue.redacted) {
    return (
      <span className={styles.redactedSummary}>
        Restricted source summary. Details are limited for this view.
      </span>
    );
  }

  const parts = [
    issue.stateName,
    issue.priorityLabel ? `${issue.priorityLabel} priority` : null,
    issue.projectName,
    issue.assigneeName ? `Assigned to ${issue.assigneeName}` : null,
    issue.dueDate ? `Due ${formatDate(issue.dueDate)}` : null,
  ].filter(Boolean);

  return <span>{parts.join(" / ")}</span>;
}

function toTrustBand(value: string): TrustBandValue | null {
  return TRUST_BANDS.includes(value as TrustBandValue)
    ? (value as TrustBandValue)
    : null;
}

function formatDate(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(parsed);
}
