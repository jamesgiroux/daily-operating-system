/**
 * useSentimentBlock — write wiring for the composition sentiment-hero block.
 *
 * Same commands the production page uses (`set_user_health_sentiment`,
 * `update_latest_sentiment_note`); the composition surface re-projects on
 * its own cadence, so the updated-detail return value is ignored here.
 */
import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SentimentValue } from "@/types";

export function useSentimentBlock(accountId?: string, current?: SentimentValue | null) {
  const onSetSentiment = useCallback(
    async (value: SentimentValue, note?: string) => {
      if (!accountId) return;
      await invoke("set_user_health_sentiment", {
        accountId,
        sentiment: value,
        note: note?.trim() ? note.trim() : null,
      });
    },
    [accountId],
  );

  const onUpdateNote = useCallback(
    async (note: string) => {
      if (!accountId) return;
      await invoke("update_latest_sentiment_note", {
        accountId,
        note: note.trim().length > 0 ? note.trim() : null,
      });
    },
    [accountId],
  );

  const onAcknowledgeStale = useCallback(async () => {
    if (!accountId || !current) return;
    await invoke("set_user_health_sentiment", {
      accountId,
      sentiment: current,
      note: null,
    });
  }, [accountId, current]);

  return { onSetSentiment, onUpdateNote, onAcknowledgeStale };
}
