import { useCallback, useEffect, useRef } from "react";

interface UseInfiniteScrollOptions {
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  threshold?: number;
}

/**
 * Infinite scroll hook using a callback ref pattern.
 *
 * The previous implementation used a useRef + useEffect([threshold]) combo.
 * This broke when the sentinel element wasn't in the DOM on the first render
 * (e.g. during a loading state), because the effect captured `null` and never
 * re-ran. Switching to a callback ref ensures the IntersectionObserver is
 * created/destroyed exactly when the sentinel mounts/unmounts.
 */
export function useInfiniteScroll({
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
  threshold = 0.1,
}: UseInfiniteScrollOptions) {
  // Keep volatile react-query status values in a ref to prevent observer teardown/recreation
  const stateRef = useRef({ hasNextPage, isFetchingNextPage, fetchNextPage });
  useEffect(() => {
    stateRef.current = { hasNextPage, isFetchingNextPage, fetchNextPage };
  });

  // Hold the current observer instance so we can disconnect it on cleanup
  const observerInstanceRef = useRef<IntersectionObserver | null>(null);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      observerInstanceRef.current?.disconnect();
    };
  }, []);

  // Callback ref: React calls this whenever the sentinel DOM node mounts/unmounts.
  // This guarantees the observer is attached even if the element appears after an
  // initial loading state (which was the root cause of the 40-image limit bug).
  const observerRef = useCallback(
    (node: HTMLDivElement | null) => {
      // Disconnect previous observer if any
      observerInstanceRef.current?.disconnect();

      if (!node) return;

      const observer = new IntersectionObserver(
        (entries) => {
          const { hasNextPage, isFetchingNextPage, fetchNextPage } = stateRef.current;
          if (entries[0].isIntersecting && hasNextPage && !isFetchingNextPage) {
            fetchNextPage();
          }
        },
        { threshold }
      );

      observer.observe(node);
      observerInstanceRef.current = observer;
    },
    [threshold]
  );

  return { observerRef };
}
