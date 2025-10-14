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
  const [error, setError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState<number>(0);
  const [upKeyCount, setUpKeyCount] = useState<number>(0);
  const containerRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);
  const upKeyTimeoutRef = useRef<number | null>(null);
  const scrollObserverRef = useRef<IntersectionObserver | null>(null);

  const getPageImage = (page: number) => {
    return "http://localhost:11337/api/picture/" + series_id + "/" + volume_id + "/" + chapter_id + "/" + page;
  };

  const loadImage = (page: number, isRetry: boolean = false) => {
    if (!isRetry) {
      setError(null);
      setRetryCount(0);
    }
    
    const img = new Image();
    img.src = getPageImage(page);
    
    // Create a timeout reference with longer timeout for retries
    const timeoutDuration = isRetry ? 15000 : 10000;
    const timeoutId = setTimeout(() => {
      if (retryCount < 3) {
        setRetryCount(prev => prev + 1);
        setError(`Image load timed out. Retrying... (${retryCount + 1}/3)`);
        // Auto-retry after a short delay
        setTimeout(() => {
          loadImage(page, true);
        }, 2000);
      } else {
        setError("Image load failed after 3 retries. The server might be unresponsive.");
      }
    }, timeoutDuration);
    
    img.onload = () => {
      clearTimeout(timeoutId);
      setError(null);
      setRetryCount(0);
    };
    
    img.onerror = () => {
      clearTimeout(timeoutId);
      if (retryCount < 3) {
        setRetryCount(prev => prev + 1);
        setError(`Failed to load image. Retrying... (${retryCount + 1}/3)`);
        // Auto-retry after a short delay
        setTimeout(() => {
          loadImage(page, true);
        }, 2000);
      } else {
        setError("Failed to load image after 3 retries. The server might be offline or the image doesn't exist.");
      }
    };
    
    return img;
  };

  // Handler for loading previous pages
  const handleLoadPrevious = () => {
    const firstPage = loadedPages[0];
    if (firstPage > 0) {
      const previousPage = firstPage - 1;
      setLoadedPages(prev => [previousPage, ...prev]);
      setUpKeyCount(0);
    }
  };

  const handleKey = (event: KeyboardEvent): void => {
    if (event.key === "Backspace") {
      navigate(-1);
    } else if (event.key === "ArrowUp") {
      // Check if user is at the top of the page
      if (window.scrollY <= 10) {
        setUpKeyCount(prev => prev + 1);
        
        // Clear existing timeout
        if (upKeyTimeoutRef.current) {
          clearTimeout(upKeyTimeoutRef.current);
        }
        
        // Set new timeout to reset counter
        upKeyTimeoutRef.current = setTimeout(() => {
          setUpKeyCount(0);
        }, 2000);
        
        // Load previous pages after 3 key presses
        if (upKeyCount >= 2) {
          handleLoadPrevious();
        }
      }
    }
  };

  useEffect(() => {
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey);
      if (upKeyTimeoutRef.current) {
        clearTimeout(upKeyTimeoutRef.current);
      }
    };
  }, [upKeyCount]);

  useEffect(() => {
    const options = {
      root: null,
      rootMargin: "0px",
      threshold: 0.1,
    };
    const handleIntersect = (entries: IntersectionObserverEntry[]) => {
      console.log('Intersection observer triggered:', entries);
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          console.log('Bottom trigger is intersecting');
          const nextPage = Math.max(...loadedPages) + 1;
          console.log('Next page to load:', nextPage, 'Total pages:', pages);
          console.log('Current loaded pages:', loadedPages);
          console.log('Next page already loaded?', loadedPages.includes(nextPage));
          if (nextPage < +pages! && !loadedPages.includes(nextPage)) {
            console.log('Loading next page:', nextPage);
            setCurrentPage(nextPage);
            setLoadedPages(prev => [...prev, nextPage]);
            loadImage(nextPage);
          } else {
            console.log('Skipping load - either page already loaded or no more pages');
          }
        }
      });
    };
    observerRef.current = new IntersectionObserver(handleIntersect, options);
    
    // Try to observe the trigger element after a short delay
    setTimeout(() => {
      const trigger = document.querySelector('[data-bottom-trigger]');
      if (trigger && observerRef.current) {
        observerRef.current.observe(trigger);
        console.log('Bottom trigger connected to observer');
      } else {
        console.log('Bottom trigger not found or observer not ready');
      }
    }, 100);
    
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [currentPage, pages, loadedPages]);

  // Observer to track which page is currently in view
  useEffect(() => {
    const options = {
      root: null,
      rootMargin: "-20% 0px -20% 0px", // Only trigger when page is in center 60% of viewport
      threshold: 0.5,
    };
    
    const handlePageIntersect = (entries: IntersectionObserverEntry[]) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const pageNumber = parseInt(entry.target.getAttribute('data-page') || '0');
          setCurrentPage(pageNumber);
        }
      });
    };
    
    scrollObserverRef.current = new IntersectionObserver(handlePageIntersect, options);
    
    // Observe all loaded pages
    const pageElements = document.querySelectorAll('[data-page]');
    pageElements.forEach(el => {
      if (scrollObserverRef.current) {
        scrollObserverRef.current.observe(el);
      }
    });
    
    return () => {
      if (scrollObserverRef.current) {
        scrollObserverRef.current.disconnect();
      }
    };
  }, [loadedPages]);

  return (
    <div className="w-full h-full bg-zinc-900 min-w-[1280px] min-h-[800px] overflow-y-auto" ref={containerRef}>
      <div className="max-w-[800px] mx-auto">
        {/* Previous page loading prompt */}
        {loadedPages[0] > 0 && (
          <div className="text-center mb-4 bg-blue-600 text-white px-4 py-2 rounded-lg shadow-lg">
            {upKeyCount === 0 && "Press ↑ 3 times to load previous pages"}
            {upKeyCount === 1 && "Press ↑ 2 more times"}
            {upKeyCount === 2 && "Press ↑ 1 more time"}
          </div>
        )}
        
        {loadedPages.map((page) => (
          <div key={page} className="mb-4 relative" data-page={page}>
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
            {/* Page number overlay */}
            <div className="absolute top-2 right-2 bg-black bg-opacity-50 text-white text-sm px-2 py-1 rounded">
              {page + 1}/{pages}
            </div>
          </div>
        ))}


        {/* Progress indicator and invisible trigger */}
        {loadedPages.length < +pages! && (
          <div className="text-center mb-8 py-4">
            <div className="text-gray-400 text-sm mb-2">
              Scroll down to load more pages ({loadedPages.length} / {pages} loaded)
            </div>
            <div 
              className="h-1 w-full"
              data-bottom-trigger
              ref={(el) => {
                if (el && observerRef.current) {
                  observerRef.current.observe(el);
                  console.log('Trigger element connected to observer');
                } else {
                  console.log('Trigger element or observer not ready:', { el: !!el, observer: !!observerRef.current });
                }
              }}
            />
          </div>
        )}

        {/* Completion indicator */}
        {loadedPages.length >= +pages! && (
          <div className="text-center mb-8 py-4">
            <div className="text-green-400 text-sm flex items-center justify-center">
              <svg className="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
              </svg>
              All pages loaded ({loadedPages.length} / {pages})
            </div>
          </div>
        )}

        {error && (
          <div className="text-red-500 text-center mt-3">
            Error: {error}
            <div className="mt-2">
              <button 
                onClick={() => {
                  setRetryCount(0);
                  loadImage(currentPage);
                }} 
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
