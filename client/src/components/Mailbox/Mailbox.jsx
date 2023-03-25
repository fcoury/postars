import { useCallback } from "react";
import { getLabel } from "../../config/folders";
import useEmails from "../../hooks/useEmails";
import { useAppState } from "../../state/AppState";
import Loading from "../Loading";
import EmailListItem from "./EmailListItem";
import EmptyState from "./EmptyState";
import "./Mailbox.css";

export default function Mailbox() {
  const { emails, isLoading, error } = useEmails();
  const { state, dispatch } = useAppState();

  const handleEmailClick = useCallback(
    async (email) => {
      dispatch({ type: "setSelectedEmail", payload: email });
    },
    [dispatch]
  );

  if (error) return <div>Error: {error.message}</div>;

  const contents =
    isLoading || state.loadingEmails ? (
      <Loading />
    ) : emails.length ? (
      <div className="email-list">
        {emails.map((email) => (
          <EmailListItem
            key={email.id}
            email={email}
            selected={state.email && state.email.id === email.id}
            onClick={() => handleEmailClick(email)}
          />
        ))}
      </div>
    ) : (
      <EmptyState />
    );

  return (
    <div className="mailbox">
      <div className="title">
        <h1>{getLabel(state.currentFolder)}</h1>
        <div className="search-box">
          <i className="far fa-search"></i>
          <input aria-label="Search" placeholder="Search" type="search" />
        </div>
      </div>
      {contents}
    </div>
  );
}
