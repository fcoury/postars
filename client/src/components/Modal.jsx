// Modal.js
import React from "react";
import styles from "./Modal.module.css";

const Modal = ({ title, content, actions, isOpen }) => {
  if (!isOpen) return null;

  return (
    <div className={styles.modalOverlay}>
      <div className={styles.modal}>
        <div className={styles.modalHeader}>{title}</div>
        <div className={styles.modalContent}>{content}</div>
        <div className={styles.modalActions}>
          {actions.map((action, index) => (
            <button
              key={index}
              onClick={action.callback}
              className={action.className}
            >
              {action.label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
};

export default Modal;

// import React, { useState } from "react";
// import Modal from "./Modal";

// const App = () => {
//   const [modalOpen, setModalOpen] = useState(false);

//   const closeModal = () => {
//     setModalOpen(false);
//   };

//   const modalActions = [
//     {
//       label: "Cancel",
//       className: "cancel-button",
//       callback: closeModal,
//     },
//     {
//       label: "Confirm",
//       className: "confirm-button",
//       callback: () => {
//         // Perform the desired action
//         closeModal();
//       },
//     },
//   ];

//   return (
//     <div>
//       <button onClick={() => setModalOpen(true)}>Open Modal</button>
//       <Modal
//         title="Modal Title"
//         content="Modal Content"
//         actions={modalActions}
//         isOpen={modalOpen}
//       />
//     </div>
//   );
// };

// export default App;
