import Actions from "./Actions";
import Body from "./Body";
import Header from "./Header";

import "./Email.css";

export default function Email() {
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
