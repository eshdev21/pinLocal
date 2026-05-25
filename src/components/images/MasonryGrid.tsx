import {
  useEffect,
  useState,
  useRef,
  useCallback,
  memo,
  ReactNode,
  CSSProperties,
} from "react";

interface Position {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MasonryGridProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => ReactNode;
  getItemSize: (item: T) => { width: number; height: number };
  gap?: number;
  className?: string;
  style?: CSSProperties;
  bufferMultiplier?: number;
  scrollContainer?: HTMLElement | React.RefObject<HTMLElement> | null;
}

const BREAKPOINTS = { default: 6, 1920: 5, 1440: 4, 1024: 3, 768: 2, 500: 1 };

function getColumnCount(width: number): number {
  const sorted = Object.entries(BREAKPOINTS)
    .filter(([k]) => k !== "default")
    .sort((a, b) => Number(a[0]) - Number(b[0]));

  for (const [breakpoint, count] of sorted) {
    if (width <= Number(breakpoint)) return count;
  }
  return BREAKPOINTS.default;
}

const MasonryItem = memo(({ children, pos }: { children: ReactNode; pos: Position }) => (
  <div
    style={{
      position: "absolute",
      left: pos.x,
      top: pos.y,
      width: pos.width,
      height: pos.height,
      willChange: "transform",
      contain: "layout style paint",
    }}
  >
    {children}
  </div>
));

MasonryItem.displayName = "MasonryItem";

export default function MasonryGrid<T>({
  items,
  renderItem,
  getItemSize,
  gap = 12,
  className = "",
  style,
  bufferMultiplier = 1.5,
  scrollContainer,
}: MasonryGridProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [positions, setPositions] = useState<Position[]>([]);
  const [containerHeight, setContainerHeight] = useState(0);
  const [visibleIndices, setVisibleIndices] = useState<Set<number>>(new Set());
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(0);

  const getScrollEl = useCallback((): HTMLElement | Window => {
    if (scrollContainer) {
      if ("current" in scrollContainer) return scrollContainer.current || window;
      return scrollContainer;
    }
    // Try to find nearest scrollable parent
    let el = containerRef.current?.parentElement;
    while (el) {
      const overflow = window.getComputedStyle(el).overflowY;
      if (overflow === "auto" || overflow === "scroll") return el;
      el = el.parentElement;
    }
    return window;
  }, [scrollContainer]);

  const calculateLayout = useCallback(() => {
    if (!containerRef.current || items.length === 0) return;

    const containerWidth = containerRef.current.offsetWidth;
    const numCols = getColumnCount(containerWidth);
    const columnWidth = (containerWidth - gap * (numCols - 1)) / numCols;
    
    const columnHeights = new Array(numCols).fill(0);
    const newPositions: Position[] = [];

    items.forEach((item) => {
      const { width, height } = getItemSize(item);
      const aspectRatio = height / width;
      const itemHeight = columnWidth * aspectRatio;
      
      const minColHeight = Math.min(...columnHeights);
      const colIndex = columnHeights.indexOf(minColHeight);
      
      const x = colIndex * (columnWidth + gap);
      const y = minColHeight;

      newPositions.push({
        x,
        y,
        width: columnWidth,
        height: itemHeight,
      });

      columnHeights[colIndex] += itemHeight + gap;
    });

    setPositions(newPositions);
    setContainerHeight(Math.max(...columnHeights));
  }, [items, getItemSize, gap]);

  // Layout calculation
  useEffect(() => {
    calculateLayout();
    const resizeObserver = new ResizeObserver(() => calculateLayout());
    if (containerRef.current) resizeObserver.observe(containerRef.current);
    return () => resizeObserver.disconnect();
  }, [calculateLayout]);

  // Scroll and Resize tracking
  useEffect(() => {
    const scrollEl = getScrollEl();
    const target = scrollEl === window ? window : (scrollEl as HTMLElement);
    
    const update = () => {
      if (scrollEl === window) {
        setScrollTop(window.scrollY);
        setViewportHeight(window.innerHeight);
      } else {
        const el = scrollEl as HTMLElement;
        setScrollTop(el.scrollTop);
        setViewportHeight(el.clientHeight);
      }
    };

    update();
    target.addEventListener("scroll", update, { passive: true });
    if (scrollEl === window) {
      window.addEventListener("resize", update);
    } else {
      const ro = new ResizeObserver(update);
      ro.observe(scrollEl as HTMLElement);
      return () => {
        target.removeEventListener("scroll", update);
        ro.disconnect();
      };
    }
    
    return () => {
      target.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, [getScrollEl]);

  // Visibility calculation
  useEffect(() => {
    if (positions.length === 0 || !containerRef.current) return;

    const scrollEl = getScrollEl();
    const containerRect = containerRef.current.getBoundingClientRect();
    
    let containerOffsetTop: number;
    if (scrollEl === window) {
      containerOffsetTop = containerRect.top + window.scrollY;
    } else {
      const el = scrollEl as HTMLElement;
      containerOffsetTop = containerRect.top - el.getBoundingClientRect().top + el.scrollTop;
    }

    const buffer = viewportHeight * bufferMultiplier;
    const start = scrollTop - containerOffsetTop - buffer;
    const end = scrollTop - containerOffsetTop + viewportHeight + buffer;

    const visible = new Set<number>();
    positions.forEach((pos, index) => {
      const itemTop = pos.y;
      const itemBottom = pos.y + pos.height;

      if (itemBottom >= start && itemTop <= end) {
        visible.add(index);
      }
    });

    setVisibleIndices(visible);
  }, [positions, scrollTop, viewportHeight, bufferMultiplier, getScrollEl]);

  return (
    <div
      ref={containerRef}
      className={className}
      style={{
        position: "relative",
        width: "100%",
        height: containerHeight,
        ...style,
      }}
    >
      {Array.from(visibleIndices).map((index) => {
        const item = items[index] as any;
        const pos = positions[index];
        if (!item || !pos) return null;

        return (
          <MasonryItem key={item.id || index} pos={pos}>
            {renderItem(item, index)}
          </MasonryItem>
        );
      })}
    </div>
  );
}
