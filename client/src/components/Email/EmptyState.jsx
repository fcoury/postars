import styles from "./EmptyState.module.css";

export default function EmptyState() {
  return (
    <div className={styles.blankState}>
      <i className={`far fa-envelope ${styles.blankStateIcon}`}></i>
      <p className={styles.blankStateText}>
        Click on an email to display its content.
      </p>
    </div>
  );
}
