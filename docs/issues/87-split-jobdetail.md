# #87 refactor(frontend): split JobDetail.tsx into tab components and an actions panel

## Status

Partly done. `src/components/job/JobHeader.tsx` is built (header block + exported
`statusConfig`). The remaining panels, hooks, page rewrite and tests are described below.

## Approach

Extract presentational panels into `src/components/job/` and mutation logic into
`src/hooks/useJobActions.ts`. `pages/JobDetail.tsx` keeps only data fetching, derived
state and layout, and must end up under 250 lines.

## Files

- `src/components/job/JobHeader.tsx` — DONE (title, status badge, budget, deadline, client).
- `src/components/job/BidList.tsx` — bids list, infinite-scroll sentinel, accept-bid button.
- `src/components/job/EscrowPanel.tsx` — escrow amount, expiry countdown, reclaim button.
- `src/components/job/DeliverablePanel.tsx` — deliverable upload modal + submitted CID link.
- `src/components/job/DisputePanel.tsx` — dispute state and resolution actions.
- `src/hooks/useJobActions.ts` — `handleBid`, `handleAcceptBid`, `handleSubmitDeliverable`,
  `handleApprove`, `handleSubmitReview`, `handleReclaimFunds` (move verbatim from the page,
  return `{ isActing, isSubmittingReview, ...handlers }`).
- `src/pages/JobDetail.tsx` — import the panels/hook, delete the moved code.

## Order

1. Move the six handlers into `useJobActions` (pure move, no behaviour change).
2. Extract `BidList`, then `EscrowPanel`, `DeliverablePanel`, `DisputePanel` one at a time,
   passing props from the page.
3. Replace the inline header JSX with `<JobHeader job={job} daysLeft={daysLeft} isExpired={isExpired} />`.
4. Confirm `wc -l src/pages/JobDetail.tsx` < 250.

## Done when

- Page file < 250 lines.
- Each panel has a render test (`*.test.tsx` with React Testing Library) asserting it
  renders its key content for a fixture job/bid.
- `pnpm lint` and `pnpm tsc --noEmit` pass.
