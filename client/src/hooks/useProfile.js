// useEmails.js
import { useQuery } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useEmails = () => {
  const { state, dispatch } = useAppState();

  const {
    data: _data,
    isLoading,
    error,
  } = useQuery("profile", () => fetchData(`/me`), {
    onFetching: () => {
      dispatch({ type: "setLoadingProfile", payload: true });
    },
    onSuccess: (profile) => {
      dispatch({ type: "setLoadingProfile", payload: false });
      dispatch({ type: "setProfile", payload: profile });
    },
    onError: () => {
      dispatch({ type: "setLoadingProfile", payload: false });
    },
  });

  return {
    profile: state.profile,
    isLoading,
    error,
  };
};

export default useEmails;
