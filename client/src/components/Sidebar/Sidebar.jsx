import { useEffect, useState } from "react";
import Menu from "../Menu";
import menuStyles from "../Menu.module.css";
import styles from "./Sidebar.module.css";

export default function Sidebar() {
  const [menuOpen, setMenuOpen] = useState(false);

  const toggleMenu = () => {
    setMenuOpen(!menuOpen);
  };

  const handleOutsideClick = (event) => {
    const menuElement = document.querySelector(`.${menuStyles.menu}`);
    const menuIconElement = document.querySelector(".far.fa-bars");

    if (
      menuElement &&
      !menuElement.contains(event.target) &&
      !menuIconElement.contains(event.target)
    ) {
      setMenuOpen(false);
    }
  };

  useEffect(() => {
    document.addEventListener("mousedown", handleOutsideClick);
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
    };
  }, []);

  return (
    <div className={styles.sidebar}>
      <ul className={styles.iconList}>
        <li onClick={toggleMenu}>
          <i className="far fa-bars"></i>
        </li>
        {menuOpen && <Menu />}
        <li className={styles.selected}>
          <i className="far fa-inbox"></i>
        </li>
        <li>
          <i className="far fa-star"></i>
        </li>
        <li>
          <i className="far fa-paper-plane"></i>
        </li>
        <li>
          <i className="far fa-user-circle"></i>
        </li>
        <li>
          <i className="far fa-exclamation-square"></i>
        </li>
        <li>
          <i className="far fa-trash-alt"></i>
        </li>
      </ul>
      <ul className={`${styles.iconList} ${styles.bottomIcon}`}>
        <li>
          <i className="far fa-moon"></i>
        </li>
        <li>
          <i className="far fa-cog"></i>
        </li>
        <li>
          <i className="far fa-question-circle"></i>
        </li>
      </ul>
    </div>
  );
}
