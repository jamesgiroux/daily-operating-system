import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import type { AgendaDraftResult } from "@/types";
import { Check, Copy } from "lucide-react";

interface UseAgendaDraftOptions {
  onError?: (message: string) => void;
}

export function useAgendaDraft({ onError }: UseAgendaDraftOptions = {}) {
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [subject, setSubject] = useState<string | null>(null);
  const [body, setBody] = useState("");

  const openDraft = useCallback(
    async (meetingId: string, contextHint?: string) => {
      setOpen(true);
      setLoading(true);
      setSubject(null);
      setBody("");
      try {
        const result = await invoke<AgendaDraftResult>(
          "generate_meeting_agenda_message_draft",
          {
            meetingId,
            contextHint: contextHint || null,
          }
        );
        setSubject(result.subject ?? null);
        setBody(result.body);
      } catch (err) {
        onError?.(
          err instanceof Error
            ? err.message
            : "Failed to generate agenda message draft"
        );
      } finally {
        setLoading(false);
      }
    },
    [onError]
  );

  return { open, setOpen, loading, subject, body, openDraft };
}

interface AgendaDraftDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  loading: boolean;
  subject: string | null;
  body: string;
}

export function AgendaDraftDialog({
  open,
  onOpenChange,
  loading,
  subject,
  body,
}: AgendaDraftDialogProps) {
  const { copied, copy } = useCopyToClipboard();

  const handleCopy = useCallback(() => {
    const fullText = subject ? `Subject: ${subject}\n\n${body}` : body;
    copy(fullText);
  }, [subject, body, copy]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Draft Agenda Message</DialogTitle>
          <DialogDescription>
            Review and copy this draft. Sending is always manual.
          </DialogDescription>
        </DialogHeader>
        {loading ? (
          <p className="text-sm text-muted-foreground">Generating draft…</p>
        ) : (
          <div className="space-y-3">
            {subject && (
              <div className="rounded-md border border-border/70 px-3 py-2">
                <p className="text-[11px] uppercase tracking-wide text-muted-foreground">
                  Subject
                </p>
                <p className="text-sm">{subject}</p>
              </div>
            )}
            <textarea
              readOnly
              value={body}
              aria-label="Draft message body"
              className="min-h-[220px] w-full rounded-md border border-border/70 bg-muted/30 p-3 text-sm leading-relaxed"
            />
            <div className="flex justify-end">
              <Button onClick={handleCopy} disabled={loading}>
                {copied ? (
                  <Check className="mr-1.5 size-3.5" />
                ) : (
                  <Copy className="mr-1.5 size-3.5" />
                )}
                {copied ? "Copied!" : "Copy Draft"}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
