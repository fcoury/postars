import Actions from "./Actions";
import Body from "./Body";
import Header from "./Header";

import { useAppState } from "../../state/AppState";
import "./Email.css";
import EmptyState from "./EmptyState";

export default function Email() {
  const { state } = useAppState();
  const { email } = state;

  if (!email) {
    return <EmptyState />;
  }

  return (
    <div className="email">
      <Actions />
      <div className="details">
        <Header />
        <Body />
      </div>
    </div>
  );
}
