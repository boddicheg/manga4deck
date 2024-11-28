import React, { useEffect, useState, useRef } from "react";
import { fetchServerStatus } from "../services/Api";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";

const Dashboard: React.FC = () => {
  const [serverStatus, setServerStatus] = useState<string>("");
  const [currentKeyPressed, setCurrentKeyPressed] = useState<string>("");
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const currentIndexRef = useRef(currentIndex);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const navigate = useNavigate();

  async function exitApp() {
    await invoke("exit_app", {});
  }

  const navigateTo = (uri: string | null | undefined) => {
    if (uri) navigate(`${uri}`);
  };

  const cycleFocus = (direction: "next" | "prev") => {
    const nextIndex =
      direction === "next"
        ? currentIndexRef.current + 1 >= 5
          ? 5 - 1
          : currentIndexRef.current + 1
        : currentIndexRef.current - 1 < 0
        ? 0
        : currentIndexRef.current - 1;

    setCurrentIndex(nextIndex);
    divRefs.current[nextIndex]?.focus();
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndexRef.current];
    const route = currentDiv?.getAttribute("data-route");
    console.log(route);
    const exit_ = async () => {
      await exitApp();
    };
    if (route == "/exit-app") exit_();
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
    <div className="w-full h-screen p-4 bg-zinc-900">
      <h1 className="text-3xl text-white font-bold mb-6 text-center">
        Server status: {serverStatus}
      </h1>
      <h1 className="text-3xl text-white font-bold mb-6 text-center">
        Key pressed: {currentKeyPressed}
      </h1>

      <div className="grid grid-cols-8 gap-4">
        <div
          key={0}
          data-route={"/shelf"}
          onClick={() => navigateTo("/shelf")}
          ref={(el) => (divRefs.current[0] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${
              currentIndex === 0
                ? "border-2 border-red-500 "
                : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Kavita</div>
          <div className="text-sm text-gray-600 text-center">
            192.168.5.73:5001
          </div>
        </div>

        <div
          key={1}
          data-route={"/"}
          onClick={() => navigateTo("/")}
          ref={(el) => (divRefs.current[1] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${
              currentIndex === 1
                ? "border-2 border-red-500 "
                : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Clean Cache</div>
          <div className="text-sm text-gray-600 text-center">Size: 1.3Gb</div>
        </div>

        <div
          key={2}
          data-route={"/"}
          onClick={() => navigateTo("/")}
          ref={(el) => (divRefs.current[2] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${
              currentIndex === 2
                ? "border-2 border-red-500 "
                : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Update</div>
          <div className="text-sm text-gray-600 text-center">Server Kavita</div>
        </div>

        <div
          key={3}
          data-route={"/"}
          onClick={() => navigateTo("/")}
          ref={(el) => (divRefs.current[3] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${
              currentIndex === 3
                ? "border-2 border-red-500 "
                : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">
            Offline mode
          </div>
          <div className="text-sm text-gray-600 text-center">
            Only cached available
          </div>
        </div>

        <div
          key={4}
          data-route={"/exit-app"}
          // onClick={() => navigateTo("/exit")}
          ref={(el) => (divRefs.current[4] = el)} // Assign ref
          tabIndex={-1} // Make it focusable but not in tab order
          className={`
            border-2 
            rounded 
            bg-gray-300
            min-h-[200px]
            min-w-[150px]
            m-3
            inline-block
            ${
              currentIndex === 4 ? "border-2 border-red-500" : "border-gray-300"
            }`}
        >
          <div className="text-lg text-center font-bold mt-12">Exit</div>
          <div className="text-sm text-gray-600 text-center">Close app</div>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
