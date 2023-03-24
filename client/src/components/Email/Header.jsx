import { useAppState } from "../../state/AppState";
import friendlyDate from "../../utils/friendlyDate";
import Avatar from "../Avatar";

export default function Header() {
  const { state } = useAppState();

  if (!state.email) return null;

  const { email } = state;

  const from = email.from.emailAddress;

  return (
    <div className="header">
      <div className="summary">
        <Avatar name={from.name} email={from.address} size={50} />
        <div className="sender-info">
          <div className="sender">{from.name}</div>
          <div className="sender-email">{from.address}</div>
        </div>
        <div className="received">{friendlyDate(email.receivedDateTime)}</div>
      </div>
      <div className="subject">{email.subject}</div>
    </div>
  );
}
1;
