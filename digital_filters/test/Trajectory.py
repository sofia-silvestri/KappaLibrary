# -*- coding: utf-8 -*-
"""
Created on Fri Nov  7 18:35:33 2025

@author: Sofia Silvestri
"""
from tkinter import Tk
from tkinter.filedialog import askdirectory
import numpy as np

def simulate_trajector(init_state, duraction, ts, destination_path):
    state_number = init_state.size
    state_transition_matrix = np.ones((state_number, state_number))
    
    for n in range(state_number):
        for m in range(n,state_number):
            state_transition_matrix[n,m] = pow(ts, (m-n))
            
    sample_points = int(np.floor(duraction/ts))+1
    state = init_state
    outputs = np.zeros(sample_points)
    print(type(state))
    print(type(state[0]))
    print(outputs.size)
    print(outputs[0])
    
    count = 0
    for t in range(sample_points):
        update_state = state_transition_matrix * state
        print(type(outputs[count]))
        outputs[count] = state[0]
        count += 1
        state = update_state
    
    outputs.tofile(destination_path)
    return 

if __name__ == "__main__":
    root = Tk()
    root.withdraw()
    
    filename = askdirectory()
    root.update()
    root.destroy()
    
    #simulate_trajector(np.array([0.0, 10.0, 4.0, 3.0], ), 100, 0.01, filename+"/ekf_input.bin")
    import struct

    with open(filename + "/ekf_noisy_spiral.bin", "rb") as f:
        contenuto = f.read()
        input_traj = np.array(struct.unpack("d" * (len(contenuto)//8), contenuto))
    with open(filename + "/ekf_noisy_spiral_output.bin", "rb") as f:
        contenuto = f.read()
        output_traj = np.array(struct.unpack("d" * (len(contenuto)//8), contenuto))
    