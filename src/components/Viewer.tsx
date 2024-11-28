import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

interface ViewerParams {
  [id: string]: string | undefined;
}

const Viewer: React.FC = () => {
  const { id, pages, read } = useParams<ViewerParams>();
  const pagesIndices = Array.from({ length: +pages! }, (_, index) => index);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [loadedImages, setLoadedImages] = useState<number>(0);
  const navigate = useNavigate();

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
    switch (event.key) {
      case "ArrowLeft":
        break;
      case "ArrowRight":
        break;
      case "Backspace":
        navigate(-1);
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  const handleImageLoad = () => {
    setLoadedImages((prevLoaded) => prevLoaded + 1);
  };

  useEffect(() => {
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  useEffect(() => {
    if (loadedImages == +pages!)
      divRefs.current[+read! - 1]?.scrollIntoView({
        behavior: "smooth",
        block: "start",
      });
  }, [loadedImages]);

  return (
    <>
      <div className="w-full h-full p-4 bg-zinc-900 min-w-[1280px]">
        <ul className="grid place-content-center">
          {pagesIndices.map((pageIdx) => (
            <li>
              <img
                ref={(el) => (divRefs.current[pageIdx] = el)}
                onLoad={handleImageLoad}
                src={
                  "http://localhost:11337/api/picture/" + id + "/" + (pageIdx + 1)
                }
                alt=""
                className="max-w-[800px]"
              />
            </li>
          ))}
        </ul>
      </div>
    </>
  );
};

export default Viewer;
