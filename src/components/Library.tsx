import React, { useEffect, useRef, useState } from "react";
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

  const getSeries = async () => {
    try {
      const data = await fetchSeries(id);
      setSeries(data);
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
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    getSeries();
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

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
      <div className="w-full h-full p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-6 text-center">
          Series
        </div>
  
        <div className="grid grid-cols-8 gap-4">
          {series.map((serie, index) => (
            <div>
              <div
                key={index}
                data-route={`/series/${serie.id}`}
                ref={(el) => (divRefs.current[index] = el)} // Assign ref
                tabIndex={-1} // Make it focusable but not in tab order
                className={`p-4 border rounded focus:outline-none ${
                  currentIndex === index
                    ? "border-2 border-red-500 "
                    : "text-white"
                }`}
                style={{
                  width: "150px",
                  height: "200px",
                  backgroundImage: `url(http://localhost:11337/api/series-cover/${serie.id})`,
                  backgroundSize: "cover",
                  backgroundPosition: "center",
                }}
              >
              </div>
              <div
                className={`text-white truncate text-center min-w-[150px] pl-1 pr-1 ${
                  serie.read === 100
                    ? "bg-green-700"
                    : serie.cached ? "bg-yellow-500" : ""
                }
                
                ${
                  currentIndex === index
                  ? "text-red-500"
                  : "text-white"
                }
                `}
              >
                {serie.title}
              </div>
              <div
                className={`text-white truncate text-center text-sm min-w-[150px] pl-1 pr-1 ${
                  serie.read === 100
                    ? "bg-green-700"
                    : serie.cached ? "bg-yellow-500" : ""
                }
                ${
                  currentIndex === index
                  ? "text-red-500"
                  : "text-white"
                }
                `}
              >
                Read: {serie.read.toFixed(1)}%
              </div>
            </div>
          ))}
        </div>
      </div>
    </>
  );
};

export default Library;
