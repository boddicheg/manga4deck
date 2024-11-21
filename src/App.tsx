import React, { useEffect, useState, useRef } from "react";
const books = [
  { id: 1, title: "Kavita", author: "F. Scott Fitzgerald" },
  { id: 2, title: "To Kill a Mockingbird", author: "Harper Lee" },
];

const App: React.FC = () => {
  const [message, setMessage] = useState<string>("");
  const [currentKeyPressed, setCurrentKeyPressed] = useState<string>("");
  const divRefs = useRef<(HTMLDivElement | null)[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);

  const cycleFocus = (direction: "next" | "prev") => {
    setCurrentIndex((prevIndex) => {
      const nextIndex =
        direction === "next"
          ? (prevIndex + 1) >= books.length ? books.length -1 : (prevIndex + 1)
          : (prevIndex - 1) < 0 ? 0 : prevIndex - 1;
      return nextIndex;
    });
    divRefs.current[currentIndex]?.focus(); // Set focus on the next element
  };

  const enterDirectory = () => {
    const currentDiv = divRefs.current[currentIndex];
    const route = currentDiv?.getAttribute('data-route');
    console.log(route)
  };

  const handleKey = (event: React.KeyboardEvent) => {
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
      default:
        console.log(`Key pressed: ${event.key}`);
    }
  };

  useEffect(() => {
    fetch("http://localhost:1337/api/status")
      .then((response) => response.json())
      .then((data) => {
        setMessage(data.status ? "active" : "off");
      });

    window.addEventListener("keydown", handleKey);

    return () => {
      window.removeEventListener("keydown", handleKey); // Clean up
    };
  }, []);

  return (
    <div className="min-h-screen bg-gray-100 p-8">
      <h1 className="text-3xl font-bold mb-6 text-center">
        Server status: {message}
      </h1>
      <h1 className="text-3xl font-bold mb-6 text-center">
        Key pressed: {currentKeyPressed}
      </h1>
      <h1 className="text-3xl font-bold mb-6 text-center">
        Current index: {currentIndex}
      </h1>

      <div className="grid grid-cols-8 gap-4">
        {books.map((book, index) => (
          <div
            key={index}
            data-route={  index }
            ref={(el) => (divRefs.current[index] = el)} // Assign ref
            tabIndex={-1} // Make it focusable but not in tab order
            className={`p-4 border rounded focus:outline-none ${
              currentIndex === index
                ? "border-blue-500 bg-blue-100"
                : "border-gray-300"
            }`}
          >
            <h2 className="text-lg font-semibold">{book.title}</h2>
            <p className="text-sm text-gray-600">{book.author}</p>
          </div>
        ))}
      </div>
    </div>
  );
};

export default App;
