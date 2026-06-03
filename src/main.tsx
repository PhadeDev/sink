import ReactDOM from "react-dom/client";
import App from "./App";
import "material-symbols/outlined.css";
import "./styles/globals.css";

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(<App />);
}
