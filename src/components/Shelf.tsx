import React, { useEffect, useState, useRef } from "react";
import { fetchLibrary, LibraryResponseInterface } from "../services/Api";
import { useNavigate } from "react-router-dom";
import { useLocalStorage } from "../services/useLocalStorage";

const Shelf: React.FC = () => {
  const [libraries, setLibrary] = useState<Array<LibraryResponseInterface>>([]);
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const { setItem, getItem } = useLocalStorage("selected_library_id");

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
    const id = currentDiv?.getAttribute("data-library-id");
    setItem(id + "")
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

  const getLibrary = async () => {
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
    getLibrary();
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  useEffect(() => {
    var selected_library_id = getItem()
    if (selected_library_id)
    {
      for (let index = 0; index < divRefs.current.length; index++) {
        const element = divRefs.current[index];
        if (element)
        {
          const id = element?.getAttribute("data-library-id");
          if (id == selected_library_id)
          {
            setCurrentIndex(index);
            divRefs.current[index]?.focus()
          }
        }
      }
    }
  }, [libraries]);

  useEffect(() => {
    currentIndexRef.current = currentIndex;
  }, [currentIndex]);

  if (loading) {
    return <>
      <div className="w-full h-screen min-w-[1280px] p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-6 text-center">
          Loading...
        </div>
      </div>
    </>
  }

  if (error) {
    return <>
      <div className="w-full h-screen min-w-[1280px] p-4 bg-zinc-900">
        <div className="text-xl text-white font-bold mb-6 text-center">
        Error: {error}
        </div>
      </div>
    </>
  }

  return (
    <>
      <div className="w-full h-screen min-w-[1280px] p-4 bg-zinc-900">
          <div className="text-xl text-white font-bold mb-1 text-center">
            Libraries
          </div>
  
        <div className="grid grid-cols-8 gap-4">
          {libraries.map((library, index) => (
            <div
              key={index}
              data-library-id={library.id}
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
            </div>
          ))}
        </div>
      </div>
    </>
  );
};

export default Shelf;
