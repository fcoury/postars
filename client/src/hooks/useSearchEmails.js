// useSearchEmails.js
import { useState } from "react";
import { useQuery } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useSearchEmails = (searchQuery) => {
  const { state, dispatch } = useAppState();
  const [isSearchLoading, setIsSearchLoading] = useState(false);

  const {
    data: _data,
    isLoading,
    error,
  } = useQuery(
    ["searchEmails", searchQuery],
    () => fetchData(`/search?q=${searchQuery}`),
    {
      enabled: !!(searchQuery && searchQuery.length > 0),
      onFetching: () => {
        setIsSearchLoading(true);
      },
      onSuccess: (emails) => {
        setIsSearchLoading(false);
        dispatch({ type: "setEmails", payload: emails });
        dispatch({ type: "setSearching", payload: true });
      },
      onError: () => {
        setIsSearchLoading(false);
      },
    }
  );

  return {
    emails: state.emails,
    isLoading: isSearchLoading,
    error,
  };
};

export default useSearchEmails;
