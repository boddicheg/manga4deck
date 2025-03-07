import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { VolumeResponseInterface, fetchCacheSeries, fetchVolumes, fetchReadVolume, fetchUnReadVolume } from "../services/Api";

interface SeriesParams {
  [id: string]: string | undefined;
}

const Series: React.FC = () => {
  const { id } = useParams<SeriesParams>();
  const [volumes, setVolumes] = useState<Array<VolumeResponseInterface>>([]);
  const volumesSizeRef = useRef(volumes.length);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);

  const fetchInterval = 1000;

  const navigate = useNavigate();
  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(uri);
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndexRef.current + 1 >= volumesSizeRef.current
          ? volumesSizeRef.current - 1
          : currentIndexRef.current + 1
        : currentIndexRef.current - 1 < 0
          ? 0
          : currentIndexRef.current - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus();
  };

  const getSeries = async () => {
    try {
      const data = await fetchVolumes(id);
      setVolumes(data);
    } catch (err) {
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError("An unexpected error occurred");
      }
    } finally {
      setLoading(false);
    }
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
    navigateTo(route);
  };

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
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
      case "F1":
        // F1 - mark volume as completed
        const readVolume = async (series_id: string | undefined, volume_id: string | undefined) => {
          await fetchReadVolume(series_id, volume_id);
        }
        const unreadVolume = async (series_id: string | undefined, volume_id: string | undefined) => {
          await fetchUnReadVolume(series_id, volume_id);
        }
        var current_volume = divRefs.current[currentIndexRef.current]?.getAttribute("data-key")
        var pages = divRefs.current[currentIndexRef.current]?.getAttribute("data-pages")
        var read = divRefs.current[currentIndexRef.current]?.getAttribute("data-read")
        if (pages == read)
          unreadVolume(id, current_volume + "")
        else
          readVolume(id, current_volume + "")
        break;
      case "F2":
        // F2 - cache whole serie
        const startCaching = async (id: string | undefined) => {
          await fetchCacheSeries(id);
        }
        startCaching(id)
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    getSeries();
    window.addEventListener("keydown", handleKey);
    const intervalId = setInterval(getSeries, fetchInterval);
    return () => {
      window.removeEventListener("keydown", handleKey);
      clearInterval(intervalId);
    };
  }, []);

  useEffect(() => {
    currentIndexRef.current = currentIndex;
    volumesSizeRef.current = volumes.length
  }, [currentIndex, volumes]);

  if (loading) {
    return <p>Loading...</p>;
  }

  if (error) {
    return <p>Error: {error}</p>;
  }

  return (
    <>
      <div className="w-full h-screen bg-[#8B5E3C]">
        <div className="text-xl text-white font-bold py-2 text-center bg-gradient-to-b from-black to-[#1a1a1a] border-b border-black shadow-md">
          Volumes
        </div>
        <div className="text-l text-white mb-2 text-center bg-black bg-opacity-50 py-1">
          F1/Y - mark volume as read, F2/X - start cache all volumes
        </div>

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
          {Array.from({ length: Math.ceil(volumes.length / 8) }).map((_, rowIndex) => (
            <div 
              key={`shelf-${rowIndex}`}
              className="absolute left-0 right-0"
              style={{ 
                height: '32px',
                top: `${rowIndex * (200 + 64) + 210}px`,
                zIndex: 1,
                background: 'linear-gradient(to bottom, #8B4513 0%, #654321 100%)',
                boxShadow: '0 2px 4px rgba(0,0,0,0.3), inset 0 1px 1px rgba(255,255,255,0.1)'
              }}
            >
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
              <div 
                className="absolute bottom-0 left-0 right-0 h-6"
                style={{
                  background: 'linear-gradient(to bottom, #654321, #4A3219)',
                  borderTop: '1px solid rgba(255,255,255,0.1)',
                  boxShadow: 'inset 0 2px 4px rgba(0,0,0,0.4)'
                }}
              />
              <div className="absolute top-0 left-0 right-0 h-[1px] bg-[rgba(255,255,255,0.15)]" />
              <div className="absolute bottom-[-4px] left-0 right-0 h-4" style={{
                background: 'linear-gradient(to bottom, rgba(0,0,0,0.3), transparent)'
              }} />
            </div>
          ))}

          {volumes.map((volume, index) => (
            <div key={index} className="flex justify-center">
              <div
                data-route={`/viewer/${volume.series_id}/${volume.volume_id}/${volume.chapter_id}/${volume.pages}/${volume.read}`}
                data-key={volume.volume_id}
                data-pages={volume.pages}
                data-read={volume.read}
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
                  backgroundImage: `url(http://localhost:11337/api/volumes-cover/${volume.volume_id})`,
                  backgroundSize: "cover",
                  backgroundPosition: "center",
                  overflow: "hidden",
                  boxShadow: currentIndex === index
                    ? "8px -8px 12px rgba(0,0,0,0.4), 3px -3px 6px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(255,255,255,0.1)"
                    : "8px -8px 8px rgba(0,0,0,0.2), 3px -3px 4px rgba(0,0,0,0.15), inset 0 0 0 1px rgba(255,255,255,0.1)"
                }}
              >
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
                      volume.read === volume.pages
                        ? "bg-green-500"
                        : volume.cached 
                        ? "bg-yellow-500"
                        : "bg-blue-500"
                    }`}
                    style={{ 
                      width: `${(volume.read / volume.pages) * 100}%`,
                      boxShadow: '0 0 4px rgba(255, 255, 255, 0.5)'
                    }}
                  />
                </div>

                <div 
                  className="absolute bottom-0 left-0 right-0"
                  style={{
                    background: volume.read === volume.pages
                      ? 'rgba(4, 120, 87, 0.7)'
                      : volume.cached
                      ? 'rgba(245, 158, 11, 0.7)'
                      : 'rgba(0, 0, 0, 0.7)',
                    backdropFilter: 'blur(5px)',
                    WebkitBackdropFilter: 'blur(5px)',
                    height: '2rem',
                    display: 'flex',
                    alignItems: 'center',
                    overflow: 'hidden',
                    justifyContent: 'center'
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
                      }
                    }}
                  >
                    {volume.title}
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

export default Series;