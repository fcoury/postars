import { useNavigate } from "react-router-dom";
import { useAppState } from "../state/AppState";
import styles from "./Menu.module.css";

export default function Menu() {
  const { dispatch } = useAppState();
  const navigate = useNavigate();

  const handleLogout = (event) => {
    event.stopPropagation();

    localStorage.removeItem("msalAccount");
    localStorage.removeItem("msalAccessToken");
    dispatch({ type: "setLoggedOut" });
    window.location.reload();
  };

  const handleProfile = (event) => {
    event.stopPropagation();
    navigate("/profile");
  };

  return (
    <div className={styles.menu}>
      <ul>
        <li onClick={handleProfile}>
          <i className="far fa-user-circle"></i>
          Profile
        </li>
        <li className={styles.separator}></li>
        <li onClick={handleLogout}>
          <i className="far fa-sign-out"></i>
          Logout
        </li>
      </ul>
    </div>
  );
}
