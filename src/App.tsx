import { RouterProvider } from "@tanstack/react-router";
import { router } from "./router";
import { UpdateProvider } from "@/contexts/UpdateContext";

function App() {
  return (
    <UpdateProvider>
      <RouterProvider router={router} />
    </UpdateProvider>
  );
}

export default App;
