import { stripHtml } from "string-strip-html";
import useEmailActions from "../../hooks/useEmailActions";
import friendlyDate from "../../utils/friendlyDate";
import Avatar from "../Avatar";
import styles from "./EmailListItem.module.css";
import LoadingSpinner from "./LoadingSpinner";

export default function EmailListItem({ email, selected, onClick }) {
  const {
    archiveEmail,
    archiveMutation,
    markAsSpam,
    spamMutation,
    toggleUnread,
    unreadMutation,
  } = useEmailActions();

  const handleArchiveClick = (event) => {
    event.stopPropagation();
    archiveEmail(email.internal_id);
  };

  const handleSpamClick = (event) => {
    event.stopPropagation();
    markAsSpam(email.internal_id);
  };

  const handleUnreadClick = () => {
    toggleUnread(email.internal_id);
  };

  const isLoading =
    archiveMutation.isLoading ||
    spamMutation.isLoading ||
    unreadMutation.isLoading;

  return (
    <div
      key={email.internal_id}
      onClick={onClick}
      className={`${styles.emailListItem}${selected ? " selected" : ""}`}
    >
      <Avatar name={email.from_name} email={email.from_addr} size={30} />
      <div className={styles.emailHeader}>
        <div className={styles.received}>
          <i className="far fa-paperclip"></i>
          {friendlyDate(email.date, true)}
        </div>
        <div className={styles.sender}>{email.from_name}</div>
        <div className={styles.subject}>{email.subject}</div>
        <div className={styles.action}>
          <i className="far fa-star"></i>
        </div>
        <div className={styles.body}>
          {email.body ? stripHtml(email.body).result : "No preview available"}
        </div>
        <div className={styles.iconContainer}>
          <i
            className={`far fa-archive ${styles.icon}`}
            onClick={handleArchiveClick}
          ></i>
          <i
            className={`far fa-exclamation-square ${styles.icon}`}
            onClick={handleSpamClick}
          ></i>
          <i
            className={`far fa-envelope-open ${styles.icon}`}
            onClick={handleUnreadClick}
          ></i>
          {isLoading && (
            <div className={styles.loadingSpinner}>
              <LoadingSpinner />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
