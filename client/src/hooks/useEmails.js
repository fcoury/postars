// useEmails.js
import React from "react";
import { useQuery } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useEmails = () => {
  const { state, dispatch } = useAppState();

  const {
    data: _data,
    isLoading,
    error,
  } = useQuery("emails", () => fetchData("/inbox/emails"), {
    onFetching: () => {
      dispatch({ type: "setLoadingEmails", payload: true });
    },
    onSuccess: (emails) => {
      dispatch({ type: "setLoadingEmails", payload: false });
      dispatch({ type: "setEmails", payload: emails });
    },
    onError: () => {
      dispatch({ type: "setLoadingEmails", payload: false });
    },
  });

  const updateEmail = (internal_id, updates) => {
    const updatedEmails = state.emails.map((email) =>
      email.internal_id === internal_id ? { ...email, ...updates } : email
    );
    dispatch({ type: "setEmails", payload: updatedEmails });
  };

  React.useEffect(() => {
    if (state.action === "updateEmail") {
      updateEmail(state.actionPayload.internal_id, state.actionPayload.updates);
    }
  }, [state.action, state.actionPayload]);

  return {
    emails: state.emails,
    isLoading,
    error,
  };
};

export default useEmails;
