import "./App.css";
import Main from "./pages/Main";
import { AppStateProvider } from "./state/AppState";

function App() {
  return (
    <AppStateProvider>
      <Main />
    </AppStateProvider>
  );
}

export default App;
