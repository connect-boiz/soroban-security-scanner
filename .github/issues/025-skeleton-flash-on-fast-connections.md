# Issue 25: [Performance] Skeleton Components Flash on Fast Connections Due to Missing Minimum Display Time

## Description

The frontend implements skeleton loading states via `SkeletonTable`, `SkeletonCard`, `SkeletonLoader`, and `LoadingSpinner` in `frontend/components/ui/`. These components appear while data is being fetched and are replaced when the data arrives. On fast connections (e.g., localhost development, fast WiFi), the data arrives so quickly (under 100ms) that the skeleton flashes briefly before being replaced. This creates a distracting "flash of loading content" (FOLC) that worsens the perceived performance experience. Research from the NN Group shows that showing a skeleton for less than 300ms feels more disruptive than showing nothing at all. The current implementation replaces the skeleton immediately when the async operation completes, with no minimum display time or smooth transition.

## Acceptance Criteria

- [ ] Implement a `minDisplayTime` prop (default 400ms) on all skeleton components that ensures they are visible for at least this duration
- [ ] Add CSS transitions (`opacity` fade with `300ms` duration) when transitioning from skeleton to content to avoid abrupt replacement
- [ ] Add a `skipSkeleton` prop that can be set to `true` for extremely fast expected queries (e.g., local validation)
- [ ] In the `useLoadingStates.ts` hook, implement the minimum display time logic so that the loading state persists for at least the configured duration
- [ ] Create a demo page in `LoadingStatesDemo.tsx` that demonstrates the skeleton display timing behavior
- [ ] Add a unit test that verifies the skeleton is visible for at least the minimum time even if data resolves immediately

## Additional Context

Key files: `frontend/components/ui/SkeletonTable.tsx`, `frontend/components/ui/SkeletonCard.tsx`, `frontend/components/ui/SkeletonLoader.tsx`, `frontend/components/ui/LoadingSpinner.tsx`, `frontend/hooks/useLoadingStates.ts`.
