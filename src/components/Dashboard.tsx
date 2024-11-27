import React, { useEffect, useState, useRef } from "react";
import { fetchServerStatus } from "../services/Api";
import { useNavigate } from "react-router-dom";

const dashboardTiles = [
    { id: 0, title: "Kavita Library"},
    { id: 1, title: "Exit"}
]

const Dashboard: React.FC = () => {
    const [serverStatus, setServerStatus] = useState<string>("");
    const [currentKeyPressed, setCurrentKeyPressed] = useState<string>("");
    const divRefs = useRef<(HTMLDivElement | null)[]>([]);
    const [currentIndex, setCurrentIndex] = useState(0);
    const currentIndexRef = useRef(currentIndex);
    const [loading, setLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);
    const navigate = useNavigate();
  
    const navigateTo = (uri: string | null | undefined) => {
        if (uri)
            navigate(`${uri}`);
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
      console.log(route);
      navigateTo(route);
    };
  
    const handleKey: (this: Window, ev: KeyboardEvent) => any = function (
      this: Window,
      event: KeyboardEvent
    ) {
      switch (event.key) {
        case "ArrowUp":
          setCurrentKeyPressed("ArrowUp");
          break;
        case "ArrowDown":
          setCurrentKeyPressed("ArrowDown");
          break;
        case "ArrowLeft":
          setCurrentKeyPressed("ArrowLeft");
          cycleFocus("prev");
          break;
        case "ArrowRight":
          setCurrentKeyPressed("ArrowRight");
          cycleFocus("next");
          break;
        case "Enter":
          setCurrentKeyPressed("Enter");
          enterDirectory();
          break;
        case "Backspace":
            setCurrentKeyPressed("Backspace");
            break;
        default:
          console.log(`Key pressed: ${event.key}`);
      }
    };
  
    const getServerStatus = async () => {
      try {
        const data = await fetchServerStatus();
        setServerStatus(data.status ? "online" : "offline");
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
      // Key press events
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
      <div className="min-h-screen bg-gray-100 p-8">
        <h1 className="text-3xl font-bold mb-6 text-center">
          Server status: {serverStatus}
        </h1>
        <h1 className="text-3xl font-bold mb-6 text-center">
          Key pressed: {currentKeyPressed}
        </h1>
  
        <div className="grid grid-cols-8 gap-4">
          <div
            key={0}
            data-route={"/shelf"}
            onClick={() => navigateTo("/shelf")}
            ref={(el) => (divRefs.current[0] = el)} // Assign ref
            tabIndex={-1} // Make it focusable but not in tab order
            className={`p-4 border rounded focus:outline-none ${
              currentIndex === 0
                ? "border-blue-500 bg-blue-100"
                : "border-gray-300"
            }`}
          >
            <h2 className="text-lg font-semibold">Kavita Library</h2>
            <p className="text-sm text-gray-600">Library size</p>
          </div>
          <div
            key={1}
            data-route={"/"}
            onClick={() => navigateTo("/")}
            ref={(el) => (divRefs.current[1] = el)} // Assign ref
            tabIndex={-1} // Make it focusable but not in tab order
            className={`p-4 border rounded focus:outline-none ${
              currentIndex === 1
                ? "border-blue-500 bg-blue-100"
                : "border-gray-300"
            }`}
          >
            <h2 className="text-lg font-semibold">Exit</h2>
            <p className="text-sm text-gray-600"></p>
          </div>
        </div>
      </div>
    );
};

export default Dashboard;