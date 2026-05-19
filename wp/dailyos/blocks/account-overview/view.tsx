import { createRoot, createElement } from "@wordpress/element";
import {
	FeedbackAffordance,
	type FeedbackAffordanceProps,
	type FeedbackRecordedDetail,
} from "../../src/components/FeedbackAffordance";

const MOUNT_SELECTOR = "[data-dailyos-feedback-affordance]";
const PROPS_ATTRIBUTE = "data-dailyos-feedback-props";

interface DailyOSFeedbackMount extends HTMLElement {
	_dailyosFeedbackMounted?: boolean;
}

function parseProps(node: HTMLElement): FeedbackAffordanceProps | null {
	const raw = node.getAttribute(PROPS_ATTRIBUTE);
	if (!raw) return null;
	try {
		const parsed = JSON.parse(raw) as FeedbackAffordanceProps;
		if (!parsed.claimId) return null;
		return parsed;
	} catch {
		return null;
	}
}

function mountFeedback(node: DailyOSFeedbackMount) {
	if (node._dailyosFeedbackMounted) return;
	const props = parseProps(node);
	if (!props) return;

	node._dailyosFeedbackMounted = true;
	const root = createRoot(node);
	root.render(
		createElement(FeedbackAffordance, {
			...props,
			onFeedbackRecorded: (detail: FeedbackRecordedDetail) => {
				node.dataset.dailyosFeedbackState = "recorded";
				node.dispatchEvent(
					new CustomEvent("dailyos:feedback-recorded", {
						bubbles: true,
						detail,
					}),
				);
			},
		}),
	);
}

function mountAll() {
	document.querySelectorAll<DailyOSFeedbackMount>(MOUNT_SELECTOR).forEach(mountFeedback);
}

if (document.readyState === "loading") {
	document.addEventListener("DOMContentLoaded", mountAll, { once: true });
} else {
	mountAll();
}
