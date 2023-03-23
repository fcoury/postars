import "./Actions.css";

export default function Actions() {
  return (
    <div className="actions">
      <div className="left">
        <i className="far fa-arrow-left"></i>
        <i className="far fa-archive"></i>
        <i className="far fa-exclamation-square"></i>
      </div>
      <div className="middle">
        <i className="far fa-chevron-left"></i>
        <div className="page">1 of 15</div>
        <i className="far fa-chevron-right"></i>
      </div>
      <div className="right">
        <i className="far fa-reply"></i>
        <i className="far fa-ellipsis-v"></i>
      </div>
    </div>
  );
}
