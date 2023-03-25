import React from "react";
import "./ToggleSwitch.css";

const ToggleSwitch = ({ isChecked, onChange }) => {
  const handleChange = (e) => {
    onChange(e.target.checked);
  };

  return (
    <label className="toggle-switch">
      <input type="checkbox" checked={isChecked} onChange={handleChange} />
      <span className="toggle-slider"></span>
    </label>
  );
};

export default ToggleSwitch;
