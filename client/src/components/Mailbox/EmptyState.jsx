import { getLabel } from "../../config/folders";
import { useAppState } from "../../state/AppState";
import styles from "./EmptyState.module.css";

export default function EmptyState() {
  const { state } = useAppState();
  return (
    <div className={styles.blankState}>
      <p className={styles.blankStateText}>
        No emails on <b>{getLabel(state.currentFolder)}</b>
      </p>
    </div>
  );
}
