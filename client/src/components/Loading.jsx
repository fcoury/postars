import styles from "./Loading.module.css";

export default function Loading() {
  return (
    <div className={styles.loadingContainer}>
      <i className="fas fa-spinner fa-spin"></i>
    </div>
  );
}
