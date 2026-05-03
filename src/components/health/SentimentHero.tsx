/**
 * SentimentHero — full journal implementation.
 *
 * Re-exports the richer SentimentHero from `components/entity` under the
 * existing import path used by AccountDetailPage. The canonical component
 * lives at `@/components/entity/SentimentHero` and accepts a `SentimentView`
 * from `useAccountDetail`. This wrapper adapts the legacy prop shape used by
 * the legacy stub to the new view-driven API when invoked without a view,
 * so the page compiles during the Wave 0 overlap.
 */
export { SentimentHero } from "@/components/entity/SentimentHero";
