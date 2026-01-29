import React, { useEffect, useState, useRef } from "react";
import { fetchLibrary, LibraryResponseInterface, fetchSeries } from "../services/Api";
import { useNavigate } from "react-router-dom";

// Function to get a random book thumbnail
const randomBookThumbnail = async (libraryId: number) => {
  try {
    const series = await fetchSeries(libraryId.toString());
    if (series && series.length > 0) {
      const randomSeries = series[Math.floor(Math.random() * series.length)];
      return `http://localhost:11337/api/series-cover/${randomSeries.id}`;
    }
  } catch (error) {
    console.error("Error fetching random book thumbnail:", error);
  }
  return '';
};

// Function to get a random manga thumbnail
const randomMangaThumbnail = async (libraryId: number) => {
  try {
    const series = await fetchSeries(libraryId.toString());
    if (series && series.length > 0) {
      const randomSeries = series[Math.floor(Math.random() * series.length)];
      return `http://localhost:11337/api/series-cover/${randomSeries.id}`;
    }
  } catch (error) {
    console.error("Error fetching random manga thumbnail:", error);
  }
  return '';
};

const Shelf: React.FC = () => {
  const [libraries, setLibrary] = useState<Array<LibraryResponseInterface>>([]);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  // Store random thumbnails for each library
  const [thumbnails, setThumbnails] = useState<{[key: string]: string}>({});

  const navigate = useNavigate();
  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(uri);
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndex + 1 >= divRefs.current.length
          ? divRefs.current.length - 1
          : currentIndex + 1
        : currentIndex - 1 < 0
        ? 0
        : currentIndex - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus(); 
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
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

  const getServerStatus = async () => {
    try {
      const data = await fetchLibrary();
      setLibrary(data);
      
      // Generate random thumbnails for each library
      const newThumbnails: {[key: string]: string} = {};
      for (const library of data) {
        if (library.title.toLowerCase().includes('book')) {
          const thumbnail = await randomBookThumbnail(library.id);
          if (thumbnail) {
            newThumbnails[library.id.toString()] = thumbnail;
          }
        } else if (library.title.toLowerCase().includes('manga')) {
          const thumbnail = await randomMangaThumbnail(library.id);
          if (thumbnail) {
            newThumbnails[library.id.toString()] = thumbnail;
          }
        }
      }
      setThumbnails(newThumbnails);
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

  useEffect(() => {
    getServerStatus();
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  useEffect(() => {
    currentIndexRef.current = currentIndex;
  }, [currentIndex]);

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
          Libraries
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
          {Array.from({ length: Math.ceil(libraries.length / 8) }).map((_, rowIndex) => (
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

          {libraries.map((library, index) => (
            <div key={library.id} className="flex justify-center">
              <div
                data-route={"/library/" + library.id}
                ref={(el) => (divRefs.current[index] = el)}
                tabIndex={-1}
                className={`relative rounded focus:outline-none ${
                  currentIndex === index
                    ? "border-2 border-black shadow-lg"
                    : "border border-black/30"
                }`}
                style={{
                  width: "150px",
                  height: "200px",
                  backgroundImage: `url(${thumbnails[library.id.toString()] || ''})`,
                  backgroundSize: 'cover',
                  backgroundPosition: 'center',
                  backgroundColor: thumbnails[library.id.toString()] ? 'transparent' : '#4A4A4A',
                  overflow: 'hidden',
                  boxShadow: currentIndex === index
                    ? "8px -8px 12px rgba(0,0,0,0.4), 3px -3px 6px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(255,255,255,0.1)"
                    : "8px -8px 8px rgba(0,0,0,0.2), 3px -3px 4px rgba(0,0,0,0.15), inset 0 0 0 1px rgba(255,255,255,0.1)"
                }}
              >
                <div 
                  className="absolute bottom-0 left-0 right-0 p-2"
                  style={{
                    background: 'rgba(0, 0, 0, 0.85)',
                    backdropFilter: 'blur(8px)',
                    WebkitBackdropFilter: 'blur(8px)',
                    borderTop: '1px solid rgba(255,255,255,0.1)'
                  }}
                >
                  <h1 className="text-lg text-center font-bold text-white">{library.title}</h1>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
};

export default Shelf;
