import { useEffect, useState } from "react";
import Menu from "../Menu";
import menuStyles from "../Menu.module.css";
import styles from "./Sidebar.module.css";

export default function Sidebar() {
  const [menuOpen, setMenuOpen] = useState(false);
  const [darkMode, setDarkMode] = useState(false);

  const toggleMenu = () => {
    setMenuOpen(!menuOpen);
  };

  const toggleDarkMode = () => {
    setDarkMode(!darkMode);
    if (!darkMode) {
      document.documentElement.style.setProperty(
        "--color-background",
        "#2c2c2e"
      );
      document.documentElement.style.setProperty(
        "--color-border-section",
        "#4a4a4e"
      );
      document.documentElement.style.setProperty("--color-icon", "#a9a9a9");
      document.documentElement.style.setProperty(
        "--color-icon-hover-background",
        "#3a3a3c"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected",
        "#5887ff"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected-background",
        "#1c1c1e"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected-border",
        "4px solid #0a84ff"
      );
      document.documentElement.style.setProperty("--color-text", "#fff");
      document.documentElement.style.setProperty(
        "--color-text-title",
        "#f5f5f5"
      );
      document.documentElement.style.setProperty(
        "--color-text-secondary",
        "#9e9e9e"
      );
      document.documentElement.style.setProperty(
        "--color-text-tertiary",
        "#6d6d6f"
      );
      document.documentElement.style.setProperty(
        "--color-selected-background",
        "#3a3a3c"
      );
      document.documentElement.style.setProperty(
        "--color-hover-background",
        "#434245"
      );
    } else {
      document.documentElement.style.setProperty("--color-background", "#fff");
      document.documentElement.style.setProperty(
        "--color-border-section",
        "#f3f3f3"
      );
      document.documentElement.style.setProperty("--color-icon", "#5f5f61");
      document.documentElement.style.setProperty(
        "--color-icon-hover-background",
        "#f7f7f7"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected",
        "#4368ae"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected-background",
        "#eef8ff"
      );
      document.documentElement.style.setProperty(
        "--color-icon-selected-border",
        "4px solid #2d62c9"
      );
      document.documentElement.style.setProperty("--color-text", "#000");
      document.documentElement.style.setProperty(
        "--color-text-title",
        "#202127"
      );
      document.documentElement.style.setProperty(
        "--color-text-secondary",
        "#606063"
      );
      document.documentElement.style.setProperty(
        "--color-text-tertiary",
        "#b3b6b7"
      );
      document.documentElement.style.setProperty(
        "--color-selected-background",
        "#f0f0f0"
      );
      document.documentElement.style.setProperty(
        "--color-hover-background",
        "#f7f7f7"
      );
    }
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
        <li onClick={toggleDarkMode}>
          <i className={`far ${darkMode ? "fa-sun" : "fa-moon"}`}></i>
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
