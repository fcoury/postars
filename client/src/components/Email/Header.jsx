import { useAppState } from "../../state/AppState";
import friendlyDate from "../../utils/friendlyDate";
import Avatar from "../Avatar";

export default function Header() {
  const { state } = useAppState();

  if (!state.email) return null;

  const { email } = state;

  return (
    <>
      <div className="summary">
        <Avatar name={email.from_name} email={email.from_addr} size={50} />
        <div className="sender-info">
          <div className="sender">{email.from_name}</div>
          <div className="sender-email">{email.from_addr}</div>
        </div>
        <div className="received">{friendlyDate(email.date)}</div>
      </div>
      <div className="subject">{email.subject}</div>
    </>
  );
}
1;
