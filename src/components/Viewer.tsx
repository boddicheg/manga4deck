import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

interface ViewerParams {
  [id: string]: string | undefined;
}

const Viewer: React.FC = () => {
  const { series_id, volume_id, chapter_id, pages, read } = useParams<ViewerParams>();
  const navigate = useNavigate();
  const [currentPage, setCurrentPage] = useState(+read!);
  const [loadedPages, setLoadedPages] = useState<number[]>([+read!]);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  const getPageImage = (page: number) => {
    return "http://localhost:11337/api/picture/" + series_id + "/" + volume_id + "/" + chapter_id + "/" + page;
  };

  const loadImage = (page: number) => {
    setLoading(true);
    setError(null);
    
    const img = new Image();
    img.src = getPageImage(page);
    img.onload = () => {
      setLoading(false);
    };
    img.onerror = () => {
      setError("Failed to load image. The server might be offline or the image doesn't exist.");
      setLoading(false);
    };
    setTimeout(() => {
      if (loading) {
        setError("Image load timed out. The server might be unresponsive.");
        setLoading(false);
      }
    }, 10000);
    return img;
  };

  const handleKey = (event: KeyboardEvent): void => {
    if (event.key === "Backspace") {
      navigate(-1);
    }
  };

  useEffect(() => {
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey);
    };
  }, []);

  useEffect(() => {
    const options = {
      root: null,
      rootMargin: "0px",
      threshold: 0.1,
    };
    const handleIntersect = (entries: IntersectionObserverEntry[]) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const nextPage = currentPage + 1;
          if (nextPage < +pages! && !loadedPages.includes(nextPage)) {
            setLoadedPages(prev => [...prev, nextPage]);
            setCurrentPage(nextPage);
          }
        }
      });
    };
    observerRef.current = new IntersectionObserver(handleIntersect, options);
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [currentPage, pages, loadedPages]);

  // Handler for loading previous 5 images
  const handleLoadPrevious = () => {
    const firstPage = loadedPages[0];
    if (firstPage > 0) {
      const start = Math.max(0, firstPage - 5);
      const newPages: number[] = [];
      for (let i = start; i < firstPage; i++) {
        newPages.push(i);
      }
      setLoadedPages(prev => [...newPages, ...prev]);
    }
  };

  return (
    <div className="w-full h-full bg-zinc-900 min-w-[1280px] min-h-[800px] overflow-y-auto" ref={containerRef}>
      <div className="max-w-[800px] mx-auto">
        <div className="text-white mb-3 text-center sticky top-0 bg-zinc-900 py-2 z-10">
          {/* Show button only if first loaded page is not 0 */}
          {loadedPages[0] > 0 && (
            <button
              onClick={handleLoadPrevious}
              className="mb-2 bg-blue-600 hover:bg-blue-800 text-white font-bold py-2 px-4 rounded"
            >
              Load previous 5 pages
            </button>
          )}
          <div>Page {currentPage} / {pages}</div>
        </div>
        
        {loadedPages.map((page) => (
          <div key={page} className="mb-4">
            <img
              src={getPageImage(page)}
              alt={`Page ${page}`}
              className="w-full"
              onLoad={() => {
                if (page === loadedPages[loadedPages.length - 1]) {
                  observerRef.current?.observe(document.querySelector(`img[alt=\"Page ${page}\"]`)!);
                }
              }}
            />
          </div>
        ))}

        {loading && (
          <div className="text-white text-center mb-4">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white mx-auto"></div>
            <div className="mt-2">Loading next page...</div>
          </div>
        )}

        {error && (
          <div className="text-red-500 text-center mt-3">
            Error: {error}
            <div className="mt-2">
              <button 
                onClick={() => loadImage(currentPage)} 
                className="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
              >
                Retry
              </button>
              <button 
                onClick={() => navigate(-1)} 
                className="bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded ml-2"
              >
                Go Back
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default Viewer;
