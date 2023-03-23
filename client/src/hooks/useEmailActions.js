import { useMutation } from "react-query";
import { useAppState } from "../state/AppState";
import { fetchData } from "./api";

const useEmailActions = () => {
  const { dispatch } = useAppState();

  const archiveMutation = useMutation(
    (id) => fetchData(`/emails/${id}/archive`, { method: "PUT" }),
    {
      onSuccess: (data, id) => {
        console.log("after success", data, id);
        dispatch({
          type: "removeEmail",
          payload: id,
        });
      },
    }
  );

  const spamMutation = useMutation(
    (id) => fetchData(`/emails/${id}/spam`, { method: "PUT" }),
    {
      onSuccess: (data, id) => {
        dispatch({
          type: "removeEmail",
          payload: id,
        });
      },
    }
  );

  const unreadMutation = useMutation(
    (id) => fetchData(`/emails/${id}/unread`, { method: "PUT" }),
    {
      onSuccess: (email, id) => {
        dispatch({
          type: "updateEmail",
          payload: { id, updates: email },
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
