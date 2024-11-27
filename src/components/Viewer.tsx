import React, { useEffect, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
// import { VolumeResponseInterface, fetchVolumes } from "../services/Api";

interface ViewerParams {
  [id: string]: string | undefined;
}

const Viewer: React.FC = () => {
  const { id, page } = useParams<ViewerParams>();
  const [currentPage, setCurrentPage] = useState(+page!);
  const pageRef = useRef(currentPage);
  const divRef = useRef<HTMLDivElement>(null);
  const [, setImageSize] = useState<{
    width: number;
    height: number;
  } | null>(null);

  const navigate = useNavigate();
  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(uri);
  };

  const cycle = (direction: "next" | "prev") => {
    const nextPage =
      direction === "next" ? pageRef.current + 1 : pageRef.current >= 0 ? pageRef.current - 1 : 0;
      setCurrentPage(nextPage);
      navigateTo("/viewer/" + id + "/" + nextPage)
  };

  const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
    this: Window,
    event: KeyboardEvent
  ) {
    switch (event.key) {
      case "ArrowLeft":
        cycle("prev");
        break;
      case "ArrowRight":
        cycle("next");
        break;
      case "Backspace":
        navigate(-1);
        break;
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {

    window.scrollTo(0, 0);

    const img = new Image();
    img.src = "http://localhost:1337/api/picture/" + id + "/" + pageRef.current;

    img.onload = () => {
      setImageSize({
        width: img.width,
        height: img.height,
      });

      if (divRef.current) {
        divRef.current.style.width = `${img.width}px`;
        divRef.current.style.height = `${img.height}px`;
      }
    };

    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  return (
    <>
      <div className="min-h-screen bg-gray-100 p-8">
        <h1 className="text-3xl font-bold mb-6 text-center">Series</h1>

        <div className="grid grid-cols-8 gap-4">
          <div
            ref={divRef}
            style={{
              backgroundImage: `url(http://localhost:1337/api/picture/${id}/${page})`,
              backgroundSize: "cover",
              backgroundPosition: "center",
            }}
          ></div>
        </div>
      </div>
    </>
  );
};

export default Viewer;
