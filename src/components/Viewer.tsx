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

  const getCurrentPageImage = () => {
    return "http://localhost:11337/api/picture/" + series_id + "/" + volume_id + "/" + chapter_id + "/" + currentPageRef.current;
  };

  const changeImageSrc = () => {
    document.getElementById("image-container")?.replaceChildren("");
    const img = new Image();
    img.src = getCurrentPageImage();
    console.log(img.src)

    img.onload = () => {
      console.log("onload: ", img.width, "x", img.height)
      document.getElementById("image-container")?.replaceChildren(img);
    };

    img.onerror = () => {
      console.error("Failed to load the image.");
    };
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
        // changeImageSrc()
        break;
      case "ArrowRight":
        cycleFocus("next")
        // changeImageSrc()
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
        <div id="image-container" className="mb-4 max-w-[800px]"></div>
      </div>
    </>
  );
};

export default Viewer;
