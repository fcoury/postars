import { useMutation } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useEmailActions = () => {
  const { dispatch } = useAppState();

  const archiveMutation = useMutation(
    (internal_id) =>
      fetchData(`/emails/${internal_id}/archive`, { method: "PUT" }),
    {
      onSuccess: (data, internal_id) => {
        dispatch({
          type: "removeEmail",
          payload: internal_id,
        });
      },
    }
  );

  const spamMutation = useMutation(
    (internal_id) =>
      fetchData(`/emails/${internal_id}/spam`, { method: "PUT" }),
    {
      onSuccess: (data, internal_id) => {
        dispatch({
          type: "removeEmail",
          payload: internal_id,
        });
      },
    }
  );

  const unreadMutation = useMutation(
    (internal_id) =>
      fetchData(`/emails/${internal_id}/unread`, { method: "PUT" }),
    {
      onSuccess: (email, internal_id) => {
        dispatch({
          type: "updateEmail",
          payload: { internal_id, updates: email },
        });
      },
    }
  );

  return {
    archiveMutation,
    spamMutation,
    unreadMutation,
    archiveEmail: archiveMutation.mutate,
    markAsSpam: spamMutation.mutate,
    toggleUnread: unreadMutation.mutate,
  };
};

export default useEmailActions;
