import React, { useEffect, useState, useRef } from "react";
import { fetchLibrary, LibraryResponseInterface } from "../services/Api";
import { useNavigate } from "react-router-dom";

const Shelf: React.FC = () => {
  const [libraries, setLibrary] = useState<Array<LibraryResponseInterface>>([]);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

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

  const getServerStatus = async () => {
    try {
      const data = await fetchLibrary();
      setLibrary(data);
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
      <div className="w-full h-screen p-4 bg-zinc-900">
        <h1 className="text-3xl text-white font-bold mb-6 text-center">
          Libraries
        </h1>
  
        <div className="grid grid-cols-8 gap-4">
          {libraries.map((library, index) => (
            <div
              key={index}
              data-route={"/library/" + library.id}
              ref={(el) => (divRefs.current[index] = el)}
              tabIndex={-1}
              className={`
              border-2 
              rounded 
              bg-gray-300
              min-h-[200px]
              min-w-[150px]
              m-3
              inline-block
              ${
                currentIndex === index
                  ? "border-2 border-red-500"
                  : "border-gray-300"
              }`}
            >
              <h1 className="text-2xl text-center font-bold mt-12">{library.title}</h1>
              <p className="text-sm text-gray-600 text-center mt-2">Test desc</p>
            </div>
          ))}
        </div>
      </div>
    </>
  );
};

export default Shelf;
