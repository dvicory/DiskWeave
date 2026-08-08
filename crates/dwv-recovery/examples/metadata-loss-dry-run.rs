//! Print the bounded Section 12.5 recovery matrix without opening any store.

fn main() {
    print!("{}", dwv_recovery::render_metadata_loss_matrix());
}
