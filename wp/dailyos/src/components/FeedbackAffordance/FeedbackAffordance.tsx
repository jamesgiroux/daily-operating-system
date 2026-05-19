import { useMemo, useState } from "react";
import styles from "./FeedbackAffordance.module.css";

export type FeedbackAffordanceState =
	| "idle"
	| "menu-open"
	| "form-open"
	| "loading"
	| "success"
	| "error";

export type FeedbackActionKind =
	| "confirm_current"
	| "mark_outdated"
	| "mark_false"
	| "wrong_subject"
	| "wrong_source"
	| "cannot_verify"
	| "needs_nuance"
	| "surface_inappropriate"
	| "not_relevant_here";

export interface FeedbackSource {
	id?: string;
	label?: string;
	source_ref?: string;
	ref?: string;
	invocation_id?: string;
	[key: string]: unknown;
}

export interface FeedbackRecordedResult {
	feedback_id?: string;
	new_verification_state?: string;
	[key: string]: unknown;
}

export type FeedbackRecordedDetail = FeedbackRecordedResult;

export interface FeedbackAffordanceProps {
	claimId: string;
	// V4-W4 binding tuple: required by the runtime's IssueNonceRequest +
	// VerifyNonceRequest parsers. Without these every WP-originated nonce
	// mint becomes HTTP 400 MalformedRequest. Source: rendered by
	// render-functions.php into the data-dailyos-feedback-props JSON.
	claimVersion: number;
	fieldPath: string;
	compositionId: string;
	compositionVersion: number;
	sources?: FeedbackSource[];
	currentSurface?: string;
	currentInvocationId?: string;
	onFeedbackRecorded?: (result: FeedbackRecordedResult) => void;
}

interface ActionDefinition {
	kind: FeedbackActionKind;
	label: string;
	description: string;
}

interface WpApiFetchOptions {
	path: string;
	method: "POST";
	data: Record<string, unknown>;
}

declare global {
	interface Window {
		wp?: {
			apiFetch?: <T = unknown>(options: WpApiFetchOptions) => Promise<T>;
		};
	}
}

const ACTIONS: ActionDefinition[] = [
	{
		kind: "confirm_current",
		label: "Still current",
		description: "Confirm this claim is accurate now.",
	},
	{
		kind: "mark_outdated",
		label: "Outdated",
		description: "It used to be true, but no longer is.",
	},
	{
		kind: "mark_false",
		label: "False",
		description: "Mark this claim as wrong.",
	},
	{
		kind: "wrong_subject",
		label: "Wrong subject",
		description: "The fact belongs somewhere else.",
	},
	{
		kind: "wrong_source",
		label: "Wrong source",
		description: "The cited source does not support it.",
	},
	{
		kind: "cannot_verify",
		label: "Cannot verify",
		description: "Ask DailyOS to corroborate it.",
	},
	{
		kind: "needs_nuance",
		label: "Needs nuance",
		description: "Provide a more precise version.",
	},
	{
		kind: "surface_inappropriate",
		label: "Wrong surface",
		description: "Hide it from this surface only.",
	},
	{
		kind: "not_relevant_here",
		label: "Not relevant here",
		description: "Deprioritize it in this context.",
	},
];

const SURFACES = [
	"account_overview",
	"entity_detail",
	"briefing",
	"meeting_prep",
	"project_detail",
];

function sourceLabel(source: FeedbackSource, index: number): string {
	if (typeof source.label === "string" && source.label.trim()) {
		return source.label;
	}
	if (typeof source.source_ref === "string" && source.source_ref.trim()) {
		return source.source_ref;
	}
	if (typeof source.ref === "string" && source.ref.trim()) {
		return source.ref;
	}
	if (typeof source.id === "string" && source.id.trim()) {
		return source.id;
	}
	return `Source ${index + 1}`;
}

function sourceRef(source: FeedbackSource | undefined, index: number): string {
	if (!source) {
		return String(index);
	}
	for (const key of ["source_ref", "ref", "id", "invocation_id"] as const) {
		const value = source[key];
		if (typeof value === "string" && value.trim()) {
			return value;
		}
	}
	return String(index);
}

function errorMessage(error: unknown): string {
	if (error && typeof error === "object") {
		const record = error as Record<string, unknown>;
		for (const key of ["rejection_reason", "reason", "message", "error"] as const) {
			const value = record[key];
			if (typeof value === "string" && value.trim()) {
				return value;
			}
		}
		const data = record.data;
		if (data && typeof data === "object") {
			const nested = data as Record<string, unknown>;
			const reason = nested.rejection_reason ?? nested.reason ?? nested.message;
			if (typeof reason === "string" && reason.trim()) {
				return reason;
			}
		}
	}
	return "Feedback was rejected. Dismiss this message and try again.";
}

export function FeedbackAffordance({
	claimId,
	claimVersion,
	fieldPath,
	compositionId,
	compositionVersion,
	sources = [],
	currentSurface,
	currentInvocationId,
	onFeedbackRecorded,
}: FeedbackAffordanceProps) {
	const [state, setState] = useState<FeedbackAffordanceState>("idle");
	const [selectedAction, setSelectedAction] = useState<ActionDefinition | null>(null);
	const [note, setNote] = useState("");
	const [sourceIndex, setSourceIndex] = useState("0");
	const [surface, setSurface] = useState(currentSurface || "account_overview");
	const [invocationId, setInvocationId] = useState(currentInvocationId || "");
	const [error, setError] = useState("");

	const invocationOptions = useMemo(() => {
		const values = new Set<string>();
		if (currentInvocationId) {
			values.add(currentInvocationId);
		}
		for (const source of sources) {
			if (typeof source.invocation_id === "string" && source.invocation_id.trim()) {
				values.add(source.invocation_id);
			}
		}
		return Array.from(values);
	}, [currentInvocationId, sources]);
	const surfaceOptions = useMemo(
		() =>
			Array.from(
				new Set(
					[currentSurface, ...SURFACES].filter(
						(value): value is string => typeof value === "string" && value.length > 0,
					),
				),
			),
		[currentSurface],
	);

	const close = () => {
		setState("idle");
		setSelectedAction(null);
		setNote("");
		setError("");
	};

	const openForm = (action: ActionDefinition) => {
		setSelectedAction(action);
		setNote("");
		setSourceIndex("0");
		setSurface(currentSurface || "account_overview");
		setInvocationId(currentInvocationId || invocationOptions[0] || "");
		setError("");
		setState("form-open");
	};

	const buildPayload = (): Record<string, unknown> | undefined => {
		if (!selectedAction) {
			return undefined;
		}
		const trimmedNote = note.trim();
		switch (selectedAction.kind) {
			case "needs_nuance":
				return { corrected_text: trimmedNote };
			case "wrong_subject":
				return trimmedNote ? { reason: trimmedNote } : undefined;
			case "wrong_source": {
				const index = Number.parseInt(sourceIndex, 10);
				const safeIndex = Number.isNaN(index) ? 0 : index;
				return {
					source_index: safeIndex,
					source_ref: sourceRef(sources[safeIndex], safeIndex),
					...(trimmedNote ? { reason: trimmedNote } : {}),
				};
			}
			case "surface_inappropriate":
				return { surface };
			case "not_relevant_here":
				return { invocation_id: invocationId };
			default:
				return undefined;
		}
	};

	const confirm = async () => {
		if (!selectedAction || !claimId) {
			return;
		}
		if (selectedAction.kind === "needs_nuance" && !note.trim()) {
			setError("What needs nuance? is required.");
			setState("error");
			return;
		}
		if (selectedAction.kind === "not_relevant_here" && !invocationId.trim()) {
			setError("Choose an invocation before confirming.");
			setState("error");
			return;
		}

		const apiFetch = window.wp?.apiFetch;
		if (!apiFetch) {
			setError("WordPress apiFetch is unavailable.");
			setState("error");
			return;
		}

		setState("loading");
		setError("");

		try {
			const payload = buildPayload();
			// V4-W4: every nonce mint requires the full binding tuple. action_kind
			// is the canonical key the WP feedback path uses; the WP plugin
			// renames it to `action` when forwarding to the runtime.
			const bindingTuple = {
				claim_id: claimId,
				field_path: fieldPath,
				claim_version: claimVersion,
				composition_id: compositionId,
				composition_version: compositionVersion,
			};
			const nonceResponse = await apiFetch<Record<string, unknown>>({
				path: "/dailyos/v1/nonce",
				method: "POST",
				data: {
					...bindingTuple,
					action_kind: selectedAction.kind,
					...(payload ? { payload_json: payload } : {}),
				},
			});
			const presenceNonce =
				nonceResponse.presence_nonce ??
				nonceResponse.nonce ??
				nonceResponse.nonce_digest;

			if (typeof presenceNonce !== "string" || !presenceNonce.trim()) {
				throw new Error("Runtime did not return a presence nonce.");
			}

			const verifyResponse = await apiFetch<Record<string, unknown>>({
				path: "/dailyos/v1/nonce/verify",
				method: "POST",
				data: {
					...bindingTuple,
					action_kind: selectedAction.kind,
					presence_nonce: presenceNonce,
				},
			});

			if (verifyResponse.ok === false) {
				throw verifyResponse;
			}

			setState("success");
			onFeedbackRecorded?.(verifyResponse as FeedbackRecordedResult);
		} catch (caught) {
			setError(errorMessage(caught));
			setState("error");
		}
	};

	const confirmDisabled =
		state === "loading" ||
		(selectedAction?.kind === "needs_nuance" && !note.trim()) ||
		(selectedAction?.kind === "not_relevant_here" && !invocationId.trim());

	return (
		<span className={styles.feedbackAffordance} data-state={state}>
			<button
				type="button"
				className={styles.trigger}
				onClick={() => setState(state === "menu-open" ? "idle" : "menu-open")}
				disabled={!claimId || state === "loading"}
				aria-expanded={state === "menu-open"}
			>
				Feedback
			</button>

			{state === "menu-open" ? (
				<div className={styles.menu} role="menu">
					{ACTIONS.map((action) => (
						<button
							key={action.kind}
							type="button"
							className={styles.menuItem}
							onClick={() => openForm(action)}
							role="menuitem"
						>
							<span className={styles.menuItemTitle}>{action.label}</span>
							<span className={styles.menuItemDescription}>{action.description}</span>
						</button>
					))}
				</div>
			) : null}

			{state === "form-open" || state === "loading" ? (
				<div className={styles.panel} role="dialog" aria-label="Feedback">
					<div className={styles.panelHeader}>
						<p className={styles.panelTitle}>{selectedAction?.label}</p>
						<p className={styles.panelCopy}>{selectedAction?.description}</p>
					</div>
					{selectedAction?.kind === "needs_nuance" ? (
						<label className={styles.field}>
							<span className={styles.label}>What needs nuance?</span>
							<textarea
								className={styles.textarea}
								value={note}
								onChange={(event) => setNote(event.currentTarget.value.slice(0, 500))}
								maxLength={500}
								required
							/>
							<span className={styles.meta}>{note.length}/500</span>
						</label>
					) : null}
					{selectedAction?.kind === "wrong_subject" ||
					selectedAction?.kind === "wrong_source" ? (
						<label className={styles.field}>
							<span className={styles.label}>Optional note</span>
							<textarea
								className={styles.textarea}
								value={note}
								onChange={(event) => setNote(event.currentTarget.value.slice(0, 500))}
								maxLength={500}
							/>
							<span className={styles.meta}>{note.length}/500</span>
						</label>
					) : null}
					{selectedAction?.kind === "wrong_source" ? (
						<label className={styles.field}>
							<span className={styles.label}>Source</span>
							<select
								className={styles.select}
								value={sourceIndex}
								onChange={(event) => setSourceIndex(event.currentTarget.value)}
							>
								{sources.length ? (
									sources.map((source, index) => (
										<option key={`${sourceLabel(source, index)}-${index}`} value={String(index)}>
											{sourceLabel(source, index)}
										</option>
									))
								) : (
									<option value="0">Source 1</option>
								)}
							</select>
						</label>
					) : null}
					{selectedAction?.kind === "surface_inappropriate" ? (
						<label className={styles.field}>
							<span className={styles.label}>Surface</span>
							<select
								className={styles.select}
								value={surface}
								onChange={(event) => setSurface(event.currentTarget.value)}
							>
								{surfaceOptions.map((value) => (
									<option key={value} value={value}>
										{value}
									</option>
								))}
							</select>
						</label>
					) : null}
					{selectedAction?.kind === "not_relevant_here" ? (
						<label className={styles.field}>
							<span className={styles.label}>Invocation</span>
							<select
								className={styles.select}
								value={invocationId}
								onChange={(event) => setInvocationId(event.currentTarget.value)}
							>
								{invocationOptions.length ? (
									invocationOptions.map((value) => (
										<option key={value} value={value}>
											{value}
										</option>
									))
								) : (
									<option value="">No invocation detected</option>
								)}
							</select>
						</label>
					) : null}
					<p className={styles.status}>{state === "loading" ? "Recording feedback..." : ""}</p>
					<div className={styles.actions}>
						<button type="button" className={styles.button} onClick={close}>
							Cancel
						</button>
						<button
							type="button"
							className={`${styles.button} ${styles.confirm}`}
							onClick={confirm}
							disabled={confirmDisabled}
						>
							Confirm
						</button>
					</div>
				</div>
			) : null}

			{state === "success" ? (
				<div className={styles.panel} role="status">
					<p className={`${styles.status} ${styles.success}`}>Feedback recorded.</p>
					<div className={styles.actions}>
						<button type="button" className={styles.dismiss} onClick={close}>
							Dismiss
						</button>
					</div>
				</div>
			) : null}

			{state === "error" ? (
				<div className={styles.panel} role="alert">
					<p className={`${styles.status} ${styles.error}`}>{error}</p>
					<div className={styles.actions}>
						<button type="button" className={styles.dismiss} onClick={close}>
							Dismiss
						</button>
					</div>
				</div>
			) : null}
		</span>
	);
}
