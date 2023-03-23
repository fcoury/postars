import { useEffect } from "react";
import { useAppState } from "../state/AppState";
import Home from "./Home";
import Login from "./Login";

function Main() {
  const { state, dispatch } = useAppState();

  useEffect(() => {
    const msalAccount = JSON.parse(localStorage.getItem("msalAccount"));
    if (msalAccount) {
      dispatch({ type: "setLoggedIn", payload: true });
    }
  }, [dispatch]);

  return (
    <div className="container">{state.isLoggedIn ? <Home /> : <Login />}</div>
  );
}

export default Main;
