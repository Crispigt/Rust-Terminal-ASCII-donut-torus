use std::io::Write;
use terminal_size::{Width, Height, terminal_size};
// Fuck it we go for a crate for the size because it seemed very annoying and hard to find what many terminals support to get size
// Here we have one way I guess to get it with escape codes but figured I don't want to spend a day figuring out how to get size
// of the window. https://vtdn.dev/docs/decset/mode2048-in-band-resize/



const radiusCirle: f32 = 1.0;
const radiusTorus: f32 = 2.0;
const offsetFromOrigo: f32 = 5.0;


const preF: f32 = (offsetFromOrigo)/((radiusCirle+radiusTorus)*4.0); // Precalc parts of f


// Camera is at origo looking down Z based on his describition, we want the screen to be closer than the obejct which is 5 away.
// The object will be a max of 5 units "tall", 2.5

const shades: [char; 11] = [' ', '.', ':', '-' , 'c', 'o', 'P', '?', '%', '@', '@'];

struct Window{
    w: u16,
    h: u16,
    frame: Vec<u8>,
    p_frame: Vec<u8>,
    buffer: Vec<u8>,
    zbuf: Vec<f32>,
}

fn main() {

    let (w,h) = match terminal_size(){
        Some((Width(w),Height(h))) => (w,h),
        None => return,
    };

    let mut win = Window{
        w: w,
        h: h,
        frame: vec![0u8; w as usize * h as usize],
        p_frame: vec![0u8; w as usize * h as usize],
        buffer: vec![0u8; w as usize * h as usize],
        zbuf: vec![0.0; w as usize * h as usize],
    };

    let mut f = preF * h as f32;
    
    let step = std::f32::consts::TAU / 400.0;
    let stepB = std::f32::consts::TAU / 280.0;

    let mut A: f32 = 0.0;
    let mut B: f32 = stepB*10.0;

    _ = clear();
    _ = hideCursor();

    loop{
        let (newW,newH) = match terminal_size(){
            Some((Width(w),Height(h))) => (w,h),
            None => return,
        };

        if ( win.w != newW || win.h != newH){
            clear();
            win = Window{
                w: newW,
                h: newH,
                frame: vec![0u8; newW as usize * newH as usize],
                p_frame: vec![0u8; newW as usize * newH as usize],
                buffer: vec![0u8; newW as usize * newH as usize],
                zbuf: vec![0.0; newW as usize * newH as usize],
            };


            f = preF * newH as f32;
        }

        //Rotate
        A = (A + step) % std::f32::consts::TAU;
        B = (B + stepB) % std::f32::consts::TAU;
        // Clearframe
        win.frame.fill(0);
        win.zbuf.fill(0.0);
        //Should I clear the terminal here? Or is it better jsut before draw? Hmmm.

        calculateAndDrawTorusFrame(A,B,&mut win,f);

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

}


fn calculateAndDrawTorusFrame(A: f32, B: f32, win: &mut Window, f: f32) {

    let (sinA, cosA) = A.sin_cos();
    let (sinB, cosB) = B.sin_cos();

    // These will have to be with in loops that go over the circels
    let stepsThetha = 300;
    let stepThetha = std::f32::consts::TAU / stepsThetha as f32;
    let stepsPhi = 500;
    let stepPhi = std::f32::consts::TAU / stepsPhi as f32; // Should be more frequent because it's larger
    for t in 0..stepsThetha{
        let thetha: f32 = t as f32 *stepThetha;
        let (sinThetha, cosThetha) = thetha.sin_cos();
        for p in 0..stepsPhi{
            let phi: f32 = p as f32 *stepPhi;
            let (sinPhi, cosPhi) = phi.sin_cos();
            let (x, y, zInv, L) = calculateTorusPos(sinA, cosA, sinB, cosB, sinThetha, cosThetha, sinPhi, cosPhi);
            let (xPrime, yPrime) = project(x,y,zInv, win.h, win.w, f);
            if xPrime < 0 || xPrime >= win.w as i32 || yPrime < 0 || yPrime >= win.h as i32{ continue; } // Clipp if outside
            if (L > 0.0){ //There is actually light hitting it correclty
                let q = (yPrime*win.w as i32 + xPrime) as usize;
                if ( zInv > win.zbuf[q]){ //Check so this is top element, also extract column major because we have flat array 
                    win.zbuf[q] = zInv;   // depth
                    let idxL = (L*9.0 / std::f32::consts::SQRT_2 + 1.0) as u8; // Convert to our 9 step luminance index, by normalizing and multiplying by 9
                    win.frame[q] = idxL;
                }
            }
        }
    }

    //drawOnlyPartThatChanges(win);
    // drawBuffer(win);
    drawOnlyBufferThatChanges(win);
}

fn calculateTorusPos(
  sinA: f32, cosA: f32, sinB: f32, cosB: f32, sinThetha: f32, cosThetha: f32, sinPhi: f32, cosPhi: f32
) -> (f32, f32, f32, f32) {

    let firstParam = radiusTorus + radiusCirle*cosThetha; 

    let x = firstParam*(cosB*cosPhi + sinA*sinB*sinPhi)-radiusCirle*cosA*sinB*sinThetha;
    let y = firstParam*(cosPhi*sinB - cosB*sinA*sinPhi)+radiusCirle*cosA*cosB*sinThetha;
    let z = offsetFromOrigo + cosA*firstParam*sinPhi + radiusCirle*sinA*sinThetha;
    let zInv = 1.0/z;

    let L = cosPhi*cosThetha*sinB-cosA*cosThetha*sinPhi-sinA*sinThetha+cosB*(cosA*sinThetha-cosThetha*sinA*sinPhi); // Not normalized fucker

    return (x,y,zInv,L);
}

fn project(x: f32, y: f32, zInv: f32, h: u16, w: u16, f: f32) -> (i32, i32){
    let xP: i32 = (x*zInv*f) as i32 + (w/2) as i32; // Shit this one was hard to remember x/z because of further away triangle, f is the calculated shorter part, there by we get the projection,
    // + half of the width because we move it into the positive only space from center of origo
    let yP: i32 = -(y*zInv*f) as i32 + (h/2) as i32; // Negative because the cursor has starts 1,1 in top left corner and goes downwards
    return (xP, yP);
}

// Very nice.
fn drawOnlyBufferThatChanges(window: &mut Window) -> std::io::Result<()>{
    window.buffer.clear();
    for y in 0..window.h{ 
        for x in 0..window.w{
            let indx = y as usize*window.w as usize + x as usize;
            if(window.frame[indx] != window.p_frame[indx]){
                let r = y + 1;
                let v = x + 1;
                window.p_frame[indx] = window.frame[indx];
                let pixel = shades[window.frame[indx] as usize];
                write!(window.buffer, "\x1b[{r};{v}H{pixel}")?;
            }
        }
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&window.buffer)?;
    stdout.flush();
    Ok(())    
}

fn drawBuffer(window: &mut Window) -> std::io::Result<()>{
    window.buffer.clear();
    for y in 0..window.h{ 
        write!(window.buffer, "\x1b[{};1H", y+1);
        for x in 0..window.w{
            let pixel = shades[window.frame[(y*window.w + x) as usize] as usize];
            write!(window.buffer, "{pixel}")?;
        }
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&window.buffer)?;
    stdout.flush();
    Ok(())
}

fn drawOnlyPartThatChanges(window: &mut Window) {
    for y in 0..window.h{ 
        for x in  0..window.w{
            let indx = (y*window.w + x) as usize;
            if(window.frame[indx] != window.buffer[indx]){
                _ = drawCurs(x,y,shades[window.frame[indx] as usize]);
                window.buffer[indx] = window.frame[indx];
            }
        }
    }
    print!("\x1b[0;0H");// Move cursor outside of window
}


// Takes a double arr of chars to draw to screen
fn draw(window: &mut Window){
    clear();
    for y in 0..window.h{ // Might have to be a vector that we take length or just the value in it
        for x in  0..window.w{
            _ = drawCurs(x,y,shades[window.frame[(y*window.w + x) as usize] as usize])
        }
    }
}

//Takes pos for where to draw with curs and char to draw
fn drawCurs(x: u16, y: u16, c: char) -> std::io::Result<()> {
    let r = y + 1;
    let v = x + 1;
    print!("\x1b[{r};{v}H{c}");
    Ok(())
}

fn clear() -> std::io::Result<()> {
    print!("\x1b[2J");
    Ok(())
}

fn hideCursor() -> std::io::Result<()>{
    print!("\x1b[?25l");
    Ok(())
}