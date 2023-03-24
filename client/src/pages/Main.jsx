import { useEffect } from "react";
import useEmailActions from "../hooks/useEmailActions";
import { useAppState } from "../state/AppState";
import Home from "./Home";
import Login from "./Login";

function Main() {
  const { state, dispatch } = useAppState();
  const { archiveEmail, markAsSpam } = useEmailActions();

  useEffect(() => {
    const id = state.email?.id;

    const handleKeyDown = (event) => {
      console.log("hit", event.key);
      switch (event.key) {
        case "k":
        case "ArrowUp":
          dispatch({ type: "previousEmail" });
          break;

        case "j":
        case "ArrowDown":
          dispatch({ type: "nextEmail" });
          break;

        case "e":
          if (!id) {
            return;
          }

          dispatch({ type: "addEmailLoading", payload: id });
          archiveEmail(id).finally(() => {
            dispatch({ type: "nextEmail" });
            dispatch({ type: "removeEmailLoading", payload: id });
          });
          break;

        case "!":
          if (!id) {
            return;
          }

          dispatch({ type: "addEmailLoading", payload: id });
          markAsSpam(id).finally(() => {
            dispatch({ type: "nextEmail" });
            dispatch({ type: "removeEmailLoading", payload: id });
          });
          break;
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [state.email, state.emails]);

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
