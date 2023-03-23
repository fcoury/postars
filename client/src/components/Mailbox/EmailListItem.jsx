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
    archiveEmail(email.id);
  };

  const handleSpamClick = (event) => {
    event.stopPropagation();
    markAsSpam(email.id);
  };

  const handleUnreadClick = () => {
    toggleUnread(email.id);
  };

  const isLoading =
    archiveMutation.isLoading ||
    spamMutation.isLoading ||
    unreadMutation.isLoading;

  const from = email.from.emailAddress;

  return (
    <div
      key={email.id}
      onClick={onClick}
      className={`${styles.emailListItem}${selected ? " selected" : ""}`}
    >
      <Avatar name={from.name} email={from.address} size={30} />
      <div className={styles.emailHeader}>
        <div className={styles.received}>
          <i className="far fa-paperclip"></i>
          {friendlyDate(email.receivedDateTime, true)}
        </div>
        <div className={styles.sender}>{from.name}</div>
        <div className={styles.subject}>{email.subject}</div>
        <div className={styles.action}>
          <i className="far fa-star"></i>
        </div>
        <div className={styles.body}>{email.bodyPreview}</div>
        <div className={styles.iconContainer}>
          {isLoading ? (
            <div className={styles.loadingSpinner}>
              <LoadingSpinner />
            </div>
          ) : (
            <>
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
            </>
          )}
        </div>
      </div>
    </div>
  );
}
