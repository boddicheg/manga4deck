import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

interface ViewerParams {
  [id: string]: string | undefined;
}

const Viewer: React.FC = () => {
  const { series_id, volume_id, chapter_id, pages, read } = useParams<ViewerParams>();
  const navigate = useNavigate();
  // Current page
  const [currentPage, setCurrentPage] = useState(+read!);
  const currentPageRef = useRef(currentPage);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState<boolean>(true);

  const getCurrentPageImage = () => {
    return "http://localhost:11337/api/picture/" + series_id + "/" + volume_id + "/" + chapter_id + "/" + currentPageRef.current;
  };

  const changeImageSrc = () => {
    setLoading(true);
    setError(null);
    
    document.getElementById("image-container")?.replaceChildren("");
    const img = new Image();
    img.src = getCurrentPageImage();
    console.log("Loading image:", img.src);

    img.onload = () => {
      console.log("Image loaded:", img.width, "x", img.height);
      document.getElementById("image-container")?.replaceChildren(img);
      setLoading(false);
    };

    img.onerror = () => {
      console.error("Failed to load the image.");
      setError("Failed to load image. The server might be offline or the image doesn't exist.");
      setLoading(false);
      
      // Create error message element
      const errorDiv = document.createElement("div");
      errorDiv.className = "text-red-500 text-center p-4 bg-zinc-800 rounded";
      errorDiv.textContent = "Failed to load image. The server might be offline or the image doesn't exist.";
      document.getElementById("image-container")?.replaceChildren(errorDiv);
    };
    
    // Set a timeout to handle cases where the image request hangs
    setTimeout(() => {
      if (loading) {
        setError("Image load timed out. The server might be unresponsive.");
        setLoading(false);
        
        const errorDiv = document.createElement("div");
        errorDiv.className = "text-yellow-500 text-center p-4 bg-zinc-800 rounded";
        errorDiv.textContent = "Image load timed out. The server might be unresponsive.";
        document.getElementById("image-container")?.replaceChildren(errorDiv);
      }
    }, 10000); // 10 second timeout
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentPageRef.current + 1 >= +pages!
          ? +pages! - 1
          : currentPageRef.current + 1
        : currentPageRef.current - 1 < 0
          ? 0
          : currentPageRef.current - 1;
    console.log(direction, nextIndex)
    setCurrentPage(nextIndex);
  };

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
    switch (event.key) {
      case "ArrowLeft":
        cycleFocus("prev")
        break;
      case "ArrowRight":
        cycleFocus("next")
        break;
      case "Backspace":
        navigate(-1);
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    changeImageSrc()
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  useEffect(() => {
    currentPageRef.current = currentPage;
    changeImageSrc();
    window.scrollTo(0, 0);
  }, [currentPage]);

  return (
    <>
      <div className="w-full h-full p-4 bg-zinc-900 min-w-[1280px] min-h-[800px] grid place-content-center">
        <div className="text-white mb-3 text-center">
          {currentPage} / {pages}
        </div>
        {loading && (
          <div className="text-white text-center mb-3">Loading image...</div>
        )}
        <div id="image-container" className="mb-4 max-w-[800px]"></div>
        {error && (
          <div className="text-red-500 text-center mt-3">
            Error: {error}
            <div className="mt-2">
              <button 
                onClick={() => changeImageSrc()} 
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
    </>
  );
};

export default Viewer;
