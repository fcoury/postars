import debounce from "lodash.debounce";
import { useCallback, useState } from "react";
import { getLabel } from "../../config/folders";
import useEmails from "../../hooks/useEmails";
import useSearchEmails from "../../hooks/useSearchEmails";
import { useAppState } from "../../state/AppState";
import Loading from "../Loading";
import EmailListItem from "./EmailListItem";
import EmptyState from "./EmptyState";
import "./Mailbox.css";

export default function Mailbox() {
  const {
    emails,
    isLoading: isFolderLoading,
    error: folderError,
  } = useEmails();
  const [searchQuery, setSearchQuery] = useState("");
  const { isLoading: isSearchLoading, error: searchError } =
    useSearchEmails(searchQuery);
  const { state, dispatch } = useAppState();

  const handleEmailClick = useCallback(
    async (email) => {
      dispatch({ type: "setSelectedEmail", payload: email });
    },
    [dispatch]
  );

  const debouncedSetSearchQuery = useCallback(
    debounce((value) => {
      setSearchQuery(value);
      if (!value || value.length === 0) {
        dispatch({ type: "setSearching", payload: false });
      }
    }, 300),
    []
  );

  const handleSearchInputChange = (event) => {
    event.preventDefault();
    debouncedSetSearchQuery(event.target.value);
  };

  const isLoading = isFolderLoading || isSearchLoading;
  const error = folderError || searchError;

  if (error) return <div>Error: {error.message}</div>;

  const contents = isLoading ? (
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
          <input
            aria-label="Search"
            placeholder="Search"
            type="search"
            onChange={handleSearchInputChange}
          />
        </div>
      </div>
      {contents}
    </div>
  );
}
