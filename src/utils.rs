use minetest_protocol::CommandRef;
use minetest_protocol::CommandDirection;

pub fn show_mt_command(command: &dyn CommandRef) {
    let dir = match command.direction() {
        CommandDirection::ToClient => "S->C",
        CommandDirection::ToServer => "C->S",
    };
    println!("[MT CMD] {} {}", dir, command.command_name());
    //println!("{} {:#?}", dir, command); // verbose
}
