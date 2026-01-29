import React, { useEffect, useRef, useState, useCallback } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { SeriesResponseInterface, fetchSeries } from "../services/Api";

interface LibraryParams {
  [id: string]: string | undefined;
}

const Library: React.FC = () => {
  const { id } = useParams<LibraryParams>();
  const [series, setSeries] = useState<Array<SeriesResponseInterface>>([]);
  const seriesSizeRef = useRef(series.length);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const firstLoadRef = useRef(true);

  const navigate = useNavigate();
  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(uri);
  };

  const GRID_COLUMNS = 8;

  const moveFocusBy = (delta: number) => {
    const size = seriesSizeRef.current;
    if (size <= 0) return;

    const nextIndex = Math.max(0, Math.min(size - 1, currentIndexRef.current + delta));
    setCurrentIndex(nextIndex);
    const element = divRefs.current[nextIndex];
    if (element) element.focus();
  };

  const getSeries = useCallback(async () => {
    try {
      // Keep current selection stable across list reordering
      const currentSelectedId =
        divRefs.current[currentIndexRef.current]
          ?.getAttribute("data-route")
          ?.split("/")
          .pop() || null;

      const data = await fetchSeries(id);
      const ordered = [
        ...data.filter((s) => s.read < 100),
        ...data.filter((s) => s.read === 100),
      ];
      setSeries(ordered);

      if (firstLoadRef.current) {
        const lastOpenedKey = `lastOpenedSeries`;
        const lastOpenedSeriesId = localStorage.getItem(lastOpenedKey);
        let idx = 0;
        if (lastOpenedSeriesId) {
          const foundIdx = ordered.findIndex(s => String(s.id) === lastOpenedSeriesId);
          if (foundIdx !== -1) idx = foundIdx;
        }
        setCurrentIndex(idx);
        setTimeout(() => {
          const element = divRefs.current[idx];
          if (element) {
            element.focus();
            // Scrolling is handled by useEffect when currentIndex changes
          }
        }, 100);
        firstLoadRef.current = false;
      } else if (currentSelectedId) {
        const foundIdx = ordered.findIndex((s) => String(s.id) === currentSelectedId);
        if (foundIdx !== -1 && foundIdx !== currentIndexRef.current) {
          setCurrentIndex(foundIdx);
        }
      }
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unexpected error occurred");
      }
    } finally {
      setLoading(false);
    }
  }, [id]);

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
    const seriesId = currentDiv?.getAttribute("data-route")?.split("/").pop();
    if (seriesId) {
      localStorage.setItem(`lastOpenedSeries`, seriesId);
    }
    navigateTo(route);
  };

  const handleKey = (event: KeyboardEvent): void => {
    switch (event.key) {
      case "ArrowLeft":
        moveFocusBy(-1);
        break;
      case "ArrowRight":
        moveFocusBy(1);
        break;
      case "ArrowUp":
        moveFocusBy(-GRID_COLUMNS);
        break;
      case "ArrowDown":
        moveFocusBy(GRID_COLUMNS);
        break;
      case "Enter":
        enterDirectory();
        break;
      case "Backspace":
        navigate(-1);
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    getSeries();
    window.addEventListener("keydown", handleKey);

    // Refresh when the app becomes active again (instead of polling).
    const handleFocus = () => {
      getSeries();
    };
    const handleVisibilityChange = () => {
      if (!document.hidden) getSeries();
    };
    window.addEventListener("focus", handleFocus);
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
      window.removeEventListener("focus", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [getSeries]); // Use getSeries as dependency (which depends on id)

  useEffect(() => {
    currentIndexRef.current = currentIndex;
    seriesSizeRef.current = series.length;
    
    // Fancy scroll animation to selected series when currentIndex changes
    if (series.length > 0) {
      const element = divRefs.current[currentIndex];
      if (element) {
        // Use a small delay to ensure DOM is updated
        const timeoutId = setTimeout(() => {
          const rect = element.getBoundingClientRect();
          const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
          const scrollLeft = window.pageXOffset || document.documentElement.scrollLeft;
          const elementTop = rect.top + scrollTop;
          const elementLeft = rect.left + scrollLeft;
          const centerY = elementTop - (window.innerHeight / 2) + (rect.height / 2);
          const centerX = elementLeft - (window.innerWidth / 2) + (rect.width / 2);
          
          // Fancy easing function for smooth animation
          const easeInOutCubic = (t: number): number => {
            return t < 0.5 
              ? 4 * t * t * t 
              : 1 - Math.pow(-2 * t + 2, 3) / 2;
          };
          
          // Animate scroll with easing
          const startY = window.pageYOffset || document.documentElement.scrollTop;
          const startX = window.pageXOffset || document.documentElement.scrollLeft;
          const targetY = Math.max(0, centerY);
          const targetX = Math.max(0, centerX);
          const distanceY = targetY - startY;
          const distanceX = targetX - startX;
          const duration = 800; // 800ms animation
          const startTime = performance.now();
          
          const animateScroll = (currentTime: number) => {
            const elapsed = currentTime - startTime;
            const progress = Math.min(elapsed / duration, 1);
            const eased = easeInOutCubic(progress);
            
            window.scrollTo({
              top: startY + distanceY * eased,
              left: startX + distanceX * eased,
              behavior: 'auto' // Use 'auto' since we're manually animating
            });
            
            if (progress < 1) {
              requestAnimationFrame(animateScroll);
            }
          };
          
          requestAnimationFrame(animateScroll);
        }, 50);
        
        return () => clearTimeout(timeoutId);
      }
    }
  }, [currentIndex, series]);

  if (loading) {
    return <p>Loading...</p>;
  }

  if (error) {
    return <p>Error: {error}</p>;
  }

  return (
    <>
      <div className="w-full h-full bg-[#8B5E3C]"> {/* Main container with darker wood background */}
        <div className="text-xl text-white font-bold py-2 text-center bg-gradient-to-b from-black to-[#1a1a1a] border-b border-black shadow-md">
          Series
        </div>
  
        {/* Wood texture background */}
        <div 
          className="absolute inset-0 pointer-events-none"
          style={{
            backgroundImage: `
              repeating-linear-gradient(
                45deg,
                rgba(0,0,0,0.05) 0px,
                rgba(0,0,0,0.05) 2px,
                transparent 2px,
                transparent 4px
              ),
              repeating-linear-gradient(
                -45deg,
                rgba(0,0,0,0.03) 0px,
                rgba(0,0,0,0.03) 2px,
                transparent 2px,
                transparent 4px
              )
            `,
            opacity: 0.4
          }}
        />
  
        {(() => {
          const inProgressSeries = series.filter((s) => s.read < 100);
          const completedSeries = series.filter((s) => s.read === 100);

          const renderSection = (
            title: string,
            items: Array<SeriesResponseInterface>,
            indexOffset: number
          ) => {
            if (items.length === 0) return null;

            return (
              <div className="mb-10">
                <div className="px-4 pt-4 text-white font-bold text-lg drop-shadow">
                  {title} <span className="text-white/70">({items.length})</span>
                </div>

                <div className="grid grid-cols-8 gap-x-4 gap-y-16 p-4 relative">
                  {Array.from({ length: Math.ceil(items.length / GRID_COLUMNS) }).map((_, rowIndex) => (
                    <div
                      key={`${title}-shelf-${rowIndex}`}
                      className="absolute left-0 right-0"
                      style={{
                        height: "32px",
                        top: `${rowIndex * (200 + 64) + 210}px`,
                        zIndex: 1,
                        background: "linear-gradient(to bottom, #8B4513 0%, #654321 100%)",
                        boxShadow: "0 2px 4px rgba(0,0,0,0.3), inset 0 1px 1px rgba(255,255,255,0.1)",
                      }}
                    >
                      <div
                        className="absolute inset-0"
                        style={{
                          backgroundImage: `
                            repeating-linear-gradient(
                              90deg,
                              transparent 0px,
                              transparent 2px,
                              rgba(0,0,0,0.1) 2px,
                              rgba(0,0,0,0.1) 4px
                            )
                          `,
                          opacity: 0.5,
                        }}
                      />
                      <div
                        className="absolute bottom-0 left-0 right-0 h-6"
                        style={{
                          background: "linear-gradient(to bottom, #654321, #4A3219)",
                          borderTop: "1px solid rgba(255,255,255,0.1)",
                          boxShadow: "inset 0 2px 4px rgba(0,0,0,0.4)",
                        }}
                      />
                      <div className="absolute top-0 left-0 right-0 h-[1px] bg-[rgba(255,255,255,0.15)]" />
                      <div
                        className="absolute bottom-[-4px] left-0 right-0 h-4"
                        style={{
                          background: "linear-gradient(to bottom, rgba(0,0,0,0.3), transparent)",
                        }}
                      />
                    </div>
                  ))}

                  {items.map((serie, localIndex) => {
                    const index = indexOffset + localIndex;
                    return (
                      <div key={serie.id} className="flex justify-center">
                        <div
                          data-route={`/series/${serie.id}`}
                          ref={(el) => (divRefs.current[index] = el)}
                          tabIndex={-1}
                          className={`relative rounded focus:outline-none ${
                            currentIndex === index ? "border-2 border-black shadow-lg" : "border border-black/30"
                          }`}
                          style={{
                            width: "150px",
                            height: "200px",
                            backgroundImage: `url(http://localhost:11337/api/series-cover/${serie.id})`,
                            backgroundSize: "cover",
                            backgroundPosition: "center",
                            overflow: "hidden",
                            boxShadow:
                              currentIndex === index
                                ? "8px -8px 12px rgba(0,0,0,0.4), 3px -3px 6px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(255,255,255,0.1)"
                                : "8px -8px 8px rgba(0,0,0,0.2), 3px -3px 4px rgba(0,0,0,0.15), inset 0 0 0 1px rgba(255,255,255,0.1)",
                          }}
                        >
                {/* Progress bar at the top */}
                <div 
                  className="absolute top-0 left-0 w-full h-1.5"
                  style={{
                    background: 'rgba(0, 0, 0, 0.7)',
                    backdropFilter: 'blur(1px)',
                    boxShadow: '0 1px 2px rgba(0, 0, 0, 0.3)'
                  }}
                >
                  <div 
                    className={`h-full ${
                      serie.read === 100
                        ? "bg-green-500"
                        : serie.cached 
                        ? "bg-yellow-500"
                        : "bg-blue-500"
                    }`}
                    style={{ 
                      width: `${serie.read}%`,
                      boxShadow: '0 0 4px rgba(255, 255, 255, 0.5)'
                    }}
                  />
                </div>

                {/* Title with blurred background */}
                <div 
                  className="absolute bottom-0 left-0 right-0"
                  style={{
                    background: serie.read === 100
                      ? 'rgba(4, 120, 87, 0.85)'
                      : serie.cached
                      ? 'rgba(245, 158, 11, 0.85)'
                      : 'rgba(0, 0, 0, 0.85)',
                    backdropFilter: 'blur(8px)',
                    WebkitBackdropFilter: 'blur(8px)',
                    height: '2rem',
                    display: 'flex',
                    alignItems: 'center',
                    overflow: 'hidden',
                    justifyContent: 'center',
                    borderTop: '1px solid rgba(255,255,255,0.1)'
                  }}
                >
                  <div 
                    className={`text-sm whitespace-nowrap text-center ${
                      currentIndex === index ? "text-gray-300" : "text-white truncate"
                    }`}
                    ref={(el) => {
                      if (el) {
                        const isOverflowing = el.scrollWidth > el.clientWidth;
                        const shouldAnimate = currentIndex === index && isOverflowing;
                        el.style.animation = shouldAnimate ? 'scrollText 10s linear infinite' : 'none';
                        el.style.paddingLeft = shouldAnimate ? '100%' : '0';
                        el.style.width = shouldAnimate ? 'auto' : '100%';
                        el.style.padding = '0 0.5rem';
                      }
                    }}
                  >
                    {serie.title}
                  </div>
                </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          };

          return (
            <>
              {renderSection("In progress", inProgressSeries, 0)}
              {renderSection("Completed", completedSeries, inProgressSeries.length)}
            </>
          );
        })()}
      </div>

      <style>
        {`
          @keyframes scrollText {
            0% {
              transform: translateX(0);
            }
            100% {
              transform: translateX(-200%);
            }
          }
        `}
      </style>
    </>
  );
};

export default Library;
