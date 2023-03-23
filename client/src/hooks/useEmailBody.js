import { useQuery } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useEmailBody = (internal_id) => {
  const { dispatch } = useAppState();

  const { data, isLoading, error } = useQuery(
    ["email", internal_id],
    () => fetchData(`/emails/${internal_id}`),
    {
      enabled: internal_id !== undefined,
      onFetch: () => {
        dispatch({ type: "setLoadingEmail", payload: true });
      },
      onSuccess: (data) => {
        dispatch({ type: "setLoadingEmail", payload: false });
        dispatch({ type: "setEmailBody", payload: data });
      },
      onError: () => {
        dispatch({ type: "setLoadingEmail", payload: false });
      },
    }
  );

  return {
    isLoading,
    error,
  };
};

export default useEmailBody;
