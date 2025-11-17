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

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndexRef.current + 1 >= seriesSizeRef.current
          ? seriesSizeRef.current - 1
          : currentIndexRef.current + 1
        : currentIndexRef.current - 1 < 0
        ? 0
        : currentIndexRef.current - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus(); 
  };

  const getSeries = useCallback(async () => {
    try {
      const data = await fetchSeries(id);
      setSeries(data);
      if (firstLoadRef.current) {
        const lastOpenedKey = `lastOpenedSeries`;
        const lastOpenedSeriesId = localStorage.getItem(lastOpenedKey);
        let idx = 0;
        if (lastOpenedSeriesId) {
          const foundIdx = data.findIndex(s => String(s.id) === lastOpenedSeriesId);
          if (foundIdx !== -1) idx = foundIdx;
        }
        setCurrentIndex(idx);
        setTimeout(() => {
          divRefs.current[idx]?.focus();
        }, 0);
        firstLoadRef.current = false;
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
        cycleFocus("prev");
        break;
      case "ArrowRight":
        cycleFocus("next");
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
    // Refresh series periodically to update progress
    const intervalId = setInterval(() => {
      getSeries();
    }, 5000); // Refresh every 5 seconds
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
      clearInterval(intervalId);
    };
  }, [getSeries]); // Use getSeries as dependency (which depends on id)

  useEffect(() => {
    currentIndexRef.current = currentIndex;
    seriesSizeRef.current = series.length
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
  
        <div className="grid grid-cols-8 gap-x-4 gap-y-16 p-4 relative">
          {/* Create wooden shelves for each row */}
          {Array.from({ length: Math.ceil(series.length / 8) }).map((_, rowIndex) => (
            <div 
              key={`shelf-${rowIndex}`}
              className="absolute left-0 right-0"
              style={{ 
                height: '32px',
                // Calculate position: 
                // - Each row is 200px (item height)
                // - Plus 64px (gap-y-16 = 4rem = 64px)
                // - Plus 10px offset from the bottom of items
                top: `${rowIndex * (200 + 64) + 210}px`,
                zIndex: 1,
                background: 'linear-gradient(to bottom, #8B4513 0%, #654321 100%)',
                boxShadow: '0 2px 4px rgba(0,0,0,0.3), inset 0 1px 1px rgba(255,255,255,0.1)'
              }}
            >
              {/* Wood grain texture overlay */}
              <div className="absolute inset-0" 
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
                  opacity: 0.5
                }}
              />
              {/* Shelf front face */}
              <div 
                className="absolute bottom-0 left-0 right-0 h-6"
                style={{
                  background: 'linear-gradient(to bottom, #654321, #4A3219)',
                  borderTop: '1px solid rgba(255,255,255,0.1)',
                  boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.4)'
                }}
              />
              {/* Shelf top highlight */}
              <div className="absolute top-0 left-0 right-0 h-[1px] bg-[rgba(255,255,255,0.15)]" />
              {/* Shelf bottom shadow */}
              <div className="absolute bottom-[-4px] left-0 right-0 h-4" style={{
                background: 'linear-gradient(to bottom, rgba(0,0,0,0.3), transparent)'
              }} />
            </div>
          ))}

          {series.map((serie, index) => (
            <div key={index} className="flex justify-center">
              <div
                data-route={`/series/${serie.id}`}
                ref={(el) => (divRefs.current[index] = el)}
                tabIndex={-1}
                className={`relative rounded focus:outline-none transform transition-all duration-200 ${
                  currentIndex === index
                    ? "border-2 border-black shadow-[0_0_15px_rgba(0,0,0,0.5)] scale-105 -translate-y-1"
                    : "hover:scale-105 hover:-translate-y-1"
                }`}
                style={{
                  width: "150px",
                  height: "200px",
                  backgroundImage: `url(http://localhost:11337/api/series-cover/${serie.id})`,
                  backgroundSize: "cover",
                  backgroundPosition: "center",
                  overflow: "hidden",
                  boxShadow: currentIndex === index
                    ? "8px -8px 12px rgba(0,0,0,0.4), 3px -3px 6px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(255,255,255,0.1)"
                    : "8px -8px 8px rgba(0,0,0,0.2), 3px -3px 4px rgba(0,0,0,0.15), inset 0 0 0 1px rgba(255,255,255,0.1)"
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
          ))}
        </div>
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
