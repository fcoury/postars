import useEmailActions from "../../hooks/useEmailActions";
import { useAppState } from "../../state/AppState";
import "./Actions.css";

export default function Actions() {
  const { state, dispatch } = useAppState();
  const { archiveEmail, markAsSpam } = useEmailActions();

  const handleArchiveClick = () => {
    const id = state.email?.id;
    dispatch({ type: "addEmailLoading", payload: id });
    archiveEmail(id).finally(() => {
      dispatch({ type: "nextEmail" });
      dispatch({ type: "removeEmailLoading", payload: id });
    });
  };

  const handleSpamClick = () => {
    const id = state.email?.id;
    dispatch({ type: "addEmailLoading", payload: id });
    markAsSpam(id).finally(() => {
      dispatch({ type: "nextEmail" });
      dispatch({ type: "removeEmailLoading", payload: id });
    });
  };

  const handlePreviousClick = () => {
    dispatch({ type: "previousEmail" });
  };

  const handleNextClick = () => {
    dispatch({ type: "nextEmail" });
  };

  const prevCss = state.currentEmailIndex === 0 ? " disabled" : "";
  const nextCss =
    state.currentEmailIndex === state.totalEmails - 1 ? " disabled" : "";

  return (
    <div className="actions">
      <div className="left">
        <i className="far fa-arrow-left"></i>
        <i className="far fa-archive" onClick={handleArchiveClick}></i>
        <i className="far fa-exclamation-square" onClick={handleSpamClick}></i>
      </div>
      <div className="middle">
        <i
          className={`far fa-chevron-left${prevCss}`}
          onClick={handlePreviousClick}
        ></i>
        <div className="page">
          {state.currentEmailIndex + 1} of {state.totalEmails}
        </div>
        <i
          className={`far fa-chevron-right${nextCss}`}
          onClick={handleNextClick}
        ></i>
      </div>
      <div className="right">
        <i className="far fa-reply"></i>
        <i className="far fa-ellipsis-v"></i>
      </div>
    </div>
  );
}
