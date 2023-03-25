import { useEffect } from "react";
import { Outlet, Route, Routes } from "react-router-dom";
import ProtectedRoute from "../components/ProtectedRoute";
import useEmailActions from "../hooks/useEmailActions";
import { useAppState } from "../state/AppState";
import Home from "./Home";
import Login from "./Login";
import Profile from "./Profile";

function Main() {
  const { state, dispatch } = useAppState();
  const { archiveEmail, markAsSpam } = useEmailActions();

  useEffect(() => {
    const id = state.email?.id;

    const handleKeyDown = (event) => {
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

        case "Escape":
          dispatch({ type: "unselectEmail" });
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
    <Routes>
      <Route element={<Layout />}>
        <Route
          path="/"
          element={
            <ProtectedRoute isLoggedIn={state.isLoggedIn}>
              <Home />
            </ProtectedRoute>
          }
        />
        <Route
          path="/profile"
          element={
            <ProtectedRoute isLoggedIn={state.isLoggedIn}>
              <Profile />
            </ProtectedRoute>
          }
        />
        <Route path="/login" element={<Login />} />
      </Route>
    </Routes>
  );
}

function Layout() {
  return (
    <div className="container">
      <Outlet />
    </div>
  );
}

export default Main;
